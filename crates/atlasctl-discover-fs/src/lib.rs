#![forbid(unsafe_code)]

use atlasctl_codes::DiagnosticCode;
use atlasctl_ports::{
    DiffError, DiffPort, DiscoverRequest, DiscoveryError, DiscoveryPort, OwnersError, OwnersPort,
};
use atlasctl_types::{
    ActiveGoalConfig, AtlasConfig, AtlasDiagnostic, AtlasEdge, AtlasId, AtlasNode, DiscoveredRepo,
    EdgeKind, NodeKind, PathSelector, Provenance, RepoDescriptor, RepoRelativePath,
};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::MetadataCommand;
use globset::Glob;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct FsDiscovery;

impl DiscoveryPort for FsDiscovery {
    fn discover(&self, request: &DiscoverRequest) -> Result<DiscoveredRepo, DiscoveryError> {
        let repo_root = canonical_utf8(&request.repo_root)?;
        if !repo_root.exists() {
            return Err(DiscoveryError::Message(format!(
                "repository root `{}` does not exist",
                repo_root
            )));
        }

        let repo_name = repo_root
            .file_name()
            .map(str::to_string)
            .unwrap_or_else(|| "repo".to_string());

        let (config, mut diagnostics) = load_config(&repo_root, request.config_path.as_ref());
        let (active_goal, active_diagnostics) = load_active_goal_manifest(&repo_root);
        diagnostics.extend(active_diagnostics);
        let config = AtlasConfig {
            active_goal,
            ..config
        };

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let files = collect_candidate_files(
            &repo_root,
            &config.discovery.roots,
            &config.discovery.ignore,
        );
        for rel_path in files {
            match classify_path(&rel_path) {
                FileKind::Fragment => {
                    let parsed = parse_fragment_file(&repo_root, &rel_path);
                    diagnostics.extend(parsed.diagnostics);
                    nodes.extend(parsed.nodes);
                    edges.extend(parsed.edges);
                }
                FileKind::Markdown => {
                    let parsed = parse_markdown_file(&repo_root, &rel_path);
                    diagnostics.extend(parsed.diagnostics);
                    nodes.extend(parsed.nodes);
                    edges.extend(parsed.edges);
                }
                FileKind::Other => {}
            }
        }

        let crate_result = discover_workspace_crates(&repo_root);
        diagnostics.extend(crate_result.diagnostics);
        nodes.extend(crate_result.nodes);

        validate_selectors(&repo_root, &nodes, &mut diagnostics);

        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        edges.sort_by(|left, right| {
            left.from
                .cmp(&right.from)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.to.cmp(&right.to))
        });

        Ok(DiscoveredRepo {
            repo: RepoDescriptor { name: repo_name },
            config,
            nodes,
            edges,
            diagnostics,
        })
    }
}

pub struct GitDiff;

impl DiffPort for GitDiff {
    fn changed_paths(
        &self,
        repo_root: &Utf8Path,
        base: &str,
        head: &str,
    ) -> Result<Vec<atlasctl_types::ChangedPath>, DiffError> {
        let output = std::process::Command::new("git")
            .current_dir(repo_root)
            .args(["diff", "--name-only", base, head])
            .output()
            .map_err(|err| DiffError::Message(format!("failed to run git: {err}")))?;

        if !output.status.success() {
            return Err(DiffError::Message(format!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let paths = stdout
            .lines()
            .map(|line| atlasctl_types::ChangedPath {
                path: atlasctl_types::RepoRelativePath::new(line),
            })
            .collect();

        Ok(paths)
    }
}

pub struct Codeowners;

impl OwnersPort for Codeowners {
    fn owners(
        &self,
        repo_root: &Utf8Path,
        paths: &[atlasctl_types::RepoRelativePath],
    ) -> Result<
        std::collections::BTreeMap<atlasctl_types::RepoRelativePath, Vec<String>>,
        OwnersError,
    > {
        let owners_file = repo_root.join("CODEOWNERS");
        let github_owners = repo_root.join(".github/CODEOWNERS");

        let path = if owners_file.exists() {
            owners_file
        } else if github_owners.exists() {
            github_owners
        } else {
            return Ok(std::collections::BTreeMap::new());
        };

        let content = fs::read_to_string(&path)
            .map_err(|err| OwnersError::Message(format!("failed to read CODEOWNERS: {err}")))?;

        let mut rules = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let pattern = parts[0];
            let owners: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

            let glob = match Glob::new(pattern) {
                Ok(g) => g.compile_matcher(),
                Err(_) => continue,
            };

            rules.push((glob, owners));
        }

        let mut result = std::collections::BTreeMap::new();
        for repo_path in paths {
            let mut matched_owners = Vec::new();
            // CODEOWNERS matches last-rule-wins usually, or we can collect all
            for (glob, owners) in &rules {
                if glob.is_match(repo_path.as_str()) {
                    matched_owners = owners.clone();
                }
            }
            if !matched_owners.is_empty() {
                result.insert(repo_path.clone(), matched_owners);
            }
        }

        Ok(result)
    }
}

fn canonical_utf8(path: &Utf8Path) -> Result<Utf8PathBuf, DiscoveryError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|err| DiscoveryError::Message(format!("failed to read cwd: {err}")))?
            .join(path)
            .try_into()
            .map_err(|_| DiscoveryError::Message(format!("non-utf8 path: {path}")))?
    };

    match fs::canonicalize(&absolute) {
        Ok(path) => Utf8PathBuf::from_path_buf(path)
            .map_err(|_| DiscoveryError::Message(format!("non-utf8 path: {absolute}"))),
        Err(_) => Ok(absolute),
    }
}

fn load_config(
    repo_root: &Utf8Path,
    explicit: Option<&Utf8PathBuf>,
) -> (AtlasConfig, Vec<AtlasDiagnostic>) {
    let mut diagnostics = Vec::new();
    let config_path = explicit
        .cloned()
        .unwrap_or_else(|| repo_root.join("atlas.toml"));

    if !config_path.exists() {
        return (AtlasConfig::default(), diagnostics);
    }

    let rel =
        relative_path(repo_root, &config_path).unwrap_or_else(|| Utf8PathBuf::from("atlas.toml"));
    match fs::read_to_string(&config_path) {
        Ok(contents) => match toml::from_str::<AtlasConfig>(&contents) {
            Ok(config) => (config, diagnostics),
            Err(err) => {
                diagnostics.push(AtlasDiagnostic::new(
                    DiagnosticCode::InvalidConfig,
                    format!("failed to parse `{}`: {err}", rel),
                    None,
                    Some(location(&rel)),
                ));
                (AtlasConfig::default(), diagnostics)
            }
        },
        Err(err) => {
            diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::InvalidConfig,
                format!("failed to read `{}`: {err}", rel),
                None,
                Some(location(&rel)),
            ));
            (AtlasConfig::default(), diagnostics)
        }
    }
}

fn load_active_goal_manifest(
    repo_root: &Utf8Path,
) -> (Option<ActiveGoalConfig>, Vec<AtlasDiagnostic>) {
    let active_path = repo_root.join(".codex/goals/active.toml");
    if !active_path.exists() {
        return (None, vec![]);
    }

    let rel_path =
        relative_path(repo_root, &active_path).unwrap_or_else(|| Utf8PathBuf::from("active.toml"));
    let contents = match fs::read_to_string(&active_path) {
        Ok(contents) => contents,
        Err(err) => {
            return (
                None,
                vec![AtlasDiagnostic::new(
                    DiagnosticCode::InvalidConfig,
                    format!("failed to read active goal manifest `{}`: {err}", rel_path),
                    None,
                    Some(location(&rel_path)),
                )],
            );
        }
    };

    match toml::from_str::<ActiveGoalConfig>(&contents) {
        Ok(active_goal) => (Some(active_goal), vec![]),
        Err(err) => (
            None,
            vec![AtlasDiagnostic::new(
                DiagnosticCode::InvalidConfig,
                format!("failed to parse active goal manifest `{}`: {err}", rel_path),
                None,
                Some(location(&rel_path)),
            )],
        ),
    }
}

fn collect_candidate_files(
    repo_root: &Utf8Path,
    roots: &[String],
    ignored: &[String],
) -> Vec<Utf8PathBuf> {
    let mut files = BTreeSet::new();

    for root in roots {
        let rel_root = Utf8PathBuf::from(root.as_str());
        let abs_root = if rel_root.as_str() == "." {
            repo_root.to_path_buf()
        } else {
            repo_root.join(&rel_root)
        };

        if !abs_root.exists() {
            continue;
        }

        if abs_root.is_file() {
            if let Some(rel) = relative_path(repo_root, &abs_root)
                && !is_ignored(&rel, ignored)
            {
                files.insert(rel);
            }
            continue;
        }

        for entry in WalkDir::new(&abs_root).sort_by_file_name() {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let path = match Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) {
                Ok(path) => path,
                Err(_) => continue,
            };

            let Some(rel) = relative_path(repo_root, &path) else {
                continue;
            };

            if is_ignored(&rel, ignored) {
                continue;
            }

            files.insert(rel);
        }
    }

    files.into_iter().collect()
}

fn is_ignored(path: &Utf8Path, ignored: &[String]) -> bool {
    path.components()
        .any(|component| ignored.iter().any(|ignored| component.as_str() == ignored))
}

#[derive(Debug)]
enum FileKind {
    Fragment,
    Markdown,
    Other,
}

fn classify_path(path: &Utf8Path) -> FileKind {
    let name = path.file_name().unwrap_or_default();
    if name.ends_with(".atlas.yaml") || name.ends_with(".atlas.yml") {
        FileKind::Fragment
    } else if name.ends_with(".md") {
        FileKind::Markdown
    } else {
        FileKind::Other
    }
}

#[derive(Default)]
struct DiscoveryBatch {
    nodes: Vec<AtlasNode>,
    edges: Vec<AtlasEdge>,
    diagnostics: Vec<AtlasDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct RawFragment {
    #[serde(default)]
    nodes: Vec<RawNode>,
    #[serde(default)]
    edges: Vec<RawEdge>,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    id: String,
    kind: String,
    title: String,
    summary: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    owns: Vec<String>,
    #[serde(default)]
    touches: Vec<String>,
    #[serde(default)]
    attrs: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct RawEdge {
    from: String,
    kind: String,
    to: String,
}

fn parse_fragment_file(repo_root: &Utf8Path, rel_path: &Utf8Path) -> DiscoveryBatch {
    let abs = repo_root.join(rel_path);
    let mut batch = DiscoveryBatch::default();

    let contents = match fs::read_to_string(&abs) {
        Ok(contents) => contents,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::DiscoveryFailure,
                format!("failed to read `{}`: {err}", rel_path),
                None,
                Some(location(rel_path)),
            ));
            return batch;
        }
    };

    let fragment = match serde_yaml::from_str::<RawFragment>(&contents) {
        Ok(fragment) => fragment,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::MalformedFragment,
                format!("failed to parse fragment `{}`: {}", rel_path, err),
                None,
                Some(location(rel_path)),
            ));
            return batch;
        }
    };

    for raw in fragment.nodes {
        match parse_node(raw, rel_path, None) {
            Ok(node) => batch.nodes.push(node),
            Err(diagnostic) => batch.diagnostics.push(diagnostic),
        }
    }

    for raw in fragment.edges {
        match parse_edge(raw, rel_path, None) {
            Ok(edge) => batch.edges.push(edge),
            Err(diagnostic) => batch.diagnostics.push(diagnostic),
        }
    }

    if batch.nodes.is_empty() && batch.edges.is_empty() {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::EmptyFragment,
            format!("fragment file `{}` contains no atlas metadata", rel_path),
            None,
            Some(location(rel_path)),
        ));
    }

    batch
}

#[derive(Debug, Deserialize, Default)]
struct FrontmatterEnvelope {
    atlas: Option<RawFrontmatterAtlas>,
}

#[derive(Debug, Deserialize, Default)]
struct RawFrontmatterAtlas {
    id: Option<String>,
    kind: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    owns: Vec<String>,
    #[serde(default)]
    touches: Vec<String>,
    #[serde(default)]
    attrs: BTreeMap<String, Value>,
    #[serde(default)]
    explains: Vec<String>,
    #[serde(default)]
    proves: Vec<String>,
    #[serde(default)]
    uses_fixture: Vec<String>,
    #[serde(default)]
    runs_with: Vec<String>,
    #[serde(default)]
    emits: Vec<String>,
    #[serde(default)]
    exercises: Vec<String>,
    #[serde(default)]
    documents: Vec<String>,
    #[serde(default)]
    belongs_to: Vec<String>,
    #[serde(default)]
    supports: Vec<String>,
    #[serde(default)]
    defines: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    decides: Vec<String>,
    #[serde(default)]
    implements: Vec<String>,
    #[serde(default)]
    active_for: Vec<String>,
    #[serde(default)]
    claims: Vec<String>,
    #[serde(default)]
    governs: Vec<String>,
    #[serde(default)]
    closes: Vec<String>,
    #[serde(default)]
    supersedes: Vec<String>,
}

fn parse_markdown_file(repo_root: &Utf8Path, rel_path: &Utf8Path) -> DiscoveryBatch {
    let abs = repo_root.join(rel_path);
    let mut batch = DiscoveryBatch::default();

    let contents = match fs::read_to_string(&abs) {
        Ok(contents) => contents,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::DiscoveryFailure,
                format!("failed to read `{}`: {err}", rel_path),
                None,
                Some(location(rel_path)),
            ));
            return batch;
        }
    };

    let Some((frontmatter, body)) = extract_frontmatter(&contents) else {
        return batch;
    };

    let envelope = match serde_yaml::from_str::<FrontmatterEnvelope>(&frontmatter) {
        Ok(envelope) => envelope,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::MalformedFragment,
                format!("failed to parse frontmatter `{}`: {err}", rel_path),
                None,
                Some(location(rel_path)),
            ));
            return batch;
        }
    };

    let Some(raw) = envelope.atlas else {
        return batch;
    };

    let Some(id) = raw.id else {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!("frontmatter in `{}` is missing `atlas.id`", rel_path),
            None,
            Some(location(rel_path)),
        ));
        return batch;
    };

    let Some(kind) = raw.kind else {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!("frontmatter in `{}` is missing `atlas.kind`", rel_path),
            None,
            Some(location(rel_path)),
        ));
        return batch;
    };

    let title = raw
        .title
        .or_else(|| first_heading(body).map(str::to_string))
        .unwrap_or_else(|| id.clone());

    let node = RawNode {
        id: id.clone(),
        kind,
        title,
        summary: raw.summary,
        paths: Vec::new(),
        owns: if !raw.owns.is_empty() {
            raw.owns
        } else if !raw.paths.is_empty() {
            raw.paths
        } else {
            vec![rel_path.as_str().to_string()]
        },
        touches: raw.touches,
        attrs: raw.attrs,
    };

    match parse_node(node, rel_path, Some(id.clone())) {
        Ok(node) => batch.nodes.push(node),
        Err(diagnostic) => batch.diagnostics.push(diagnostic),
    }

    for target in raw.explains {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Explains, &target, rel_path);
    }
    for target in raw.proves {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Proves, &target, rel_path);
    }
    for target in raw.uses_fixture {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::UsesFixture, &target, rel_path);
    }
    for target in raw.runs_with {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::RunsWith, &target, rel_path);
    }
    for target in raw.emits {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Emits, &target, rel_path);
    }
    for target in raw.exercises {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Exercises, &target, rel_path);
    }
    for target in raw.documents {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Documents, &target, rel_path);
    }
    for target in raw.belongs_to {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::BelongsTo, &target, rel_path);
    }
    for target in raw.supports {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Supports, &target, rel_path);
    }
    for target in raw.defines {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Defines, &target, rel_path);
    }
    for target in raw.requires {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Requires, &target, rel_path);
    }
    for target in raw.decides {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Decides, &target, rel_path);
    }
    for target in raw.implements {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Implements, &target, rel_path);
    }
    for target in raw.active_for {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::ActiveFor, &target, rel_path);
    }
    for target in raw.claims {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Claims, &target, rel_path);
    }
    for target in raw.governs {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Governs, &target, rel_path);
    }
    for target in raw.closes {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Closes, &target, rel_path);
    }
    for target in raw.supersedes {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Supersedes, &target, rel_path);
    }

    batch
}

fn push_frontmatter_edge(
    batch: &mut DiscoveryBatch,
    from: &str,
    kind: EdgeKind,
    to: &str,
    rel_path: &Utf8Path,
) {
    match (
        AtlasId::parse(from.to_string()),
        AtlasId::parse(to.to_string()),
    ) {
        (Ok(from), Ok(to)) => batch.edges.push(AtlasEdge {
            from,
            kind,
            to,
            provenance: Provenance::new(RepoRelativePath::new(rel_path.as_str())),
        }),
        _ => batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::InvalidId,
            format!("invalid edge IDs in `{}`", rel_path),
            None,
            Some(location(rel_path)),
        )),
    }
}

fn parse_node(
    raw: RawNode,
    rel_path: &Utf8Path,
    fragment: Option<String>,
) -> Result<AtlasNode, AtlasDiagnostic> {
    let id = AtlasId::parse(raw.id.clone()).map_err(|err| {
        AtlasDiagnostic::new(
            DiagnosticCode::InvalidId,
            err.to_string(),
            None,
            Some(location(rel_path)),
        )
    })?;

    let kind = raw.kind.parse::<NodeKind>().map_err(|invalid| {
        AtlasDiagnostic::new(
            DiagnosticCode::UnknownNodeKind,
            format!("unknown node kind `{invalid}` in `{}`", rel_path),
            Some(id.clone()),
            Some(location(rel_path)),
        )
    })?;

    let mut provenance = Provenance::new(RepoRelativePath::new(rel_path.as_str()));
    provenance.fragment = fragment;

    let mut owns = raw.owns;
    if owns.is_empty() {
        owns = raw.paths;
    }

    Ok(AtlasNode {
        id,
        role: kind.role(),
        kind,
        title: raw.title,
        summary: raw.summary,
        owns: owns.into_iter().map(PathSelector::new).collect(),
        touches: raw.touches.into_iter().map(PathSelector::new).collect(),
        attrs: raw.attrs,
        provenance,
    })
}

fn parse_edge(
    raw: RawEdge,
    rel_path: &Utf8Path,
    fragment: Option<String>,
) -> Result<AtlasEdge, AtlasDiagnostic> {
    let from = AtlasId::parse(raw.from.clone()).map_err(|err| {
        AtlasDiagnostic::new(
            DiagnosticCode::InvalidId,
            err.to_string(),
            None,
            Some(location(rel_path)),
        )
    })?;

    let to = AtlasId::parse(raw.to.clone()).map_err(|err| {
        AtlasDiagnostic::new(
            DiagnosticCode::InvalidId,
            err.to_string(),
            None,
            Some(location(rel_path)),
        )
    })?;

    let kind = raw.kind.parse::<EdgeKind>().map_err(|invalid| {
        AtlasDiagnostic::new(
            DiagnosticCode::UnknownEdgeKind,
            format!("unknown edge kind `{invalid}` in `{}`", rel_path),
            Some(from.clone()),
            Some(location(rel_path)),
        )
    })?;

    let mut provenance = Provenance::new(RepoRelativePath::new(rel_path.as_str()));
    provenance.fragment = fragment;

    Ok(AtlasEdge {
        from,
        kind,
        to,
        provenance,
    })
}

fn validate_selectors(
    repo_root: &Utf8Path,
    nodes: &[AtlasNode],
    diagnostics: &mut Vec<AtlasDiagnostic>,
) {
    let mut all_paths = Vec::new();
    for entry in WalkDir::new(repo_root) {
        let Ok(entry) = entry else { continue };
        if let Ok(path) = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
            && let Some(rel) = relative_path(repo_root, &path)
        {
            all_paths.push(rel);
        }
    }

    let mut file_owners: BTreeMap<String, Vec<AtlasId>> = BTreeMap::new();

    for node in nodes {
        // Validation for exclusive ownership
        for selector in &node.owns {
            let pattern = RepoRelativePath::new(selector.pattern.clone());
            let glob = match Glob::new(pattern.as_str()) {
                Ok(g) => g.compile_matcher(),
                Err(err) => {
                    diagnostics.push(AtlasDiagnostic::new(
                        DiagnosticCode::InvalidPath,
                        format!(
                            "invalid path selector pattern `{}`: {}",
                            selector.pattern, err
                        ),
                        Some(node.id.clone()),
                        Some(node.provenance.location()),
                    ));
                    continue;
                }
            };

            let mut matched = false;
            for path in &all_paths {
                let path_str = path.as_str().replace('\\', "/");
                if glob.is_match(&path_str) {
                    matched = true;
                    file_owners
                        .entry(path_str.clone())
                        .or_default()
                        .push(node.id.clone());
                }
            }

            if !matched {
                diagnostics.push(AtlasDiagnostic::new(
                    DiagnosticCode::DeadSelector,
                    format!("path selector `{}` matches no files", selector.pattern),
                    Some(node.id.clone()),
                    Some(node.provenance.location()),
                ));
            }
        }

        // Validation for non-exclusive participation (just check for dead selectors)
        for selector in &node.touches {
            let pattern = RepoRelativePath::new(selector.pattern.clone());
            let glob = match Glob::new(pattern.as_str()) {
                Ok(g) => g.compile_matcher(),
                Err(err) => {
                    diagnostics.push(AtlasDiagnostic::new(
                        DiagnosticCode::InvalidPath,
                        format!(
                            "invalid path selector pattern `{}`: {}",
                            selector.pattern, err
                        ),
                        Some(node.id.clone()),
                        Some(node.provenance.location()),
                    ));
                    continue;
                }
            };

            let matched = all_paths
                .iter()
                .any(|p| glob.is_match(p.as_str().replace('\\', "/")));
            if !matched {
                diagnostics.push(AtlasDiagnostic::new(
                    DiagnosticCode::DeadSelector,
                    format!(
                        "participation path selector `{}` matches no files",
                        selector.pattern
                    ),
                    Some(node.id.clone()),
                    Some(node.provenance.location()),
                ));
            }
        }
    }

    for (file, owners) in file_owners {
        if owners.len() > 1 {
            let mut unique_owners = owners.clone();
            unique_owners.sort();
            unique_owners.dedup();

            if unique_owners.len() > 1 {
                for owner in &unique_owners {
                    diagnostics.push(AtlasDiagnostic::new(
                        DiagnosticCode::DuplicateOwnership,
                        format!(
                            "path `{}` is claimed by multiple nodes: {}",
                            file,
                            unique_owners
                                .iter()
                                .map(|id| id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        Some(owner.clone()),
                        None,
                    ));
                }
            }
        }
    }
}

fn discover_workspace_crates(repo_root: &Utf8Path) -> DiscoveryBatch {
    let manifest = repo_root.join("Cargo.toml");
    if !manifest.exists() {
        return DiscoveryBatch::default();
    }

    let mut batch = DiscoveryBatch::default();
    let metadata = MetadataCommand::new()
        .current_dir(repo_root.as_std_path())
        .manifest_path(manifest.clone())
        .no_deps()
        .exec();

    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::DiscoveryFailure,
                format!("failed to read cargo metadata: {err}"),
                None,
                Some(location(&Utf8PathBuf::from("Cargo.toml"))),
            ));
            return batch;
        }
    };

    let workspace_members: BTreeSet<_> = metadata.workspace_members.into_iter().collect();
    for package in metadata.packages {
        if !workspace_members.contains(&package.id) {
            continue;
        }

        let crate_id = match AtlasId::parse(format!("crate:{}", package.name)) {
            Ok(id) => id,
            Err(err) => {
                batch.diagnostics.push(AtlasDiagnostic::new(
                    DiagnosticCode::InvalidId,
                    err.to_string(),
                    None,
                    Some(location(&Utf8PathBuf::from("Cargo.toml"))),
                ));
                continue;
            }
        };

        // Convert manifest_path to Utf8PathBuf for forward slash normalization
        // cargo_metadata::Package::manifest_path is already Utf8PathBuf
        let manifest_path = package.manifest_path.clone();

        let manifest_dir = manifest_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| manifest_path.clone());

        let rel_dir =
            relative_path(repo_root, &manifest_dir).unwrap_or_else(|| Utf8PathBuf::from(""));

        // Store manifest_path as relative to repository root for deterministic output
        let rel_manifest_path = relative_path(repo_root, &manifest_path)
            .unwrap_or_else(|| Utf8PathBuf::from("Cargo.toml"));

        // Normalize to forward slashes for cross-platform deterministic output
        let manifest_path_normalized = RepoRelativePath::new(rel_manifest_path.as_str());

        let mut attrs = BTreeMap::new();
        attrs.insert(
            "manifest_path".to_string(),
            Value::String(manifest_path_normalized.to_string()),
        );
        attrs.insert(
            "version".to_string(),
            Value::String(package.version.to_string()),
        );

        batch.nodes.push(AtlasNode {
            id: crate_id,
            kind: NodeKind::Crate,
            role: NodeKind::Crate.role(),
            title: package.name,
            summary: None,
            owns: vec![PathSelector::new(rel_dir.as_str())],
            touches: Vec::new(),
            attrs,
            provenance: Provenance::new(RepoRelativePath::new("Cargo.toml")),
        });
    }

    batch
}

fn relative_path(repo_root: &Utf8Path, abs_path: &Utf8Path) -> Option<Utf8PathBuf> {
    abs_path
        .strip_prefix(repo_root)
        .ok()
        .map(|path| path.to_path_buf())
}

fn location(path: &Utf8Path) -> atlasctl_types::SourceLocation {
    atlasctl_types::SourceLocation {
        path: RepoRelativePath::new(path.as_str()),
        line: None,
        column: None,
    }
}

fn extract_frontmatter(contents: &str) -> Option<(String, &str)> {
    let mut lines = contents.lines();
    if lines.next()? != "---" {
        return None;
    }

    let mut frontmatter = Vec::new();
    let mut byte_offset = 4; // initial ---\n

    for line in contents.lines().skip(1) {
        if line == "---" {
            let body = &contents[byte_offset + 4..];
            return Some((frontmatter.join("\n"), body));
        }
        frontmatter.push(line.to_string());
        byte_offset += line.len() + 1;
    }

    None
}

fn first_heading(contents: &str) -> Option<&str> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_frontmatter() {
        let input = "---\natlas:\n  id: guide:test\n  kind: guide\n---\n# Heading\n";
        let (frontmatter, body) = extract_frontmatter(input).expect("frontmatter");
        assert!(frontmatter.contains("guide:test"));
        assert!(body.contains("# Heading"));
    }

    #[test]
    fn classifies_files() {
        assert!(matches!(
            classify_path(Utf8Path::new("atlas/example.atlas.yaml")),
            FileKind::Fragment
        ));
        assert!(matches!(
            classify_path(Utf8Path::new("docs/architecture.md")),
            FileKind::Markdown
        ));
    }

    #[test]
    fn parses_source_truth_frontmatter_relations() {
        let mut root = std::env::temp_dir();
        root.push(format!("atlasctl-discover-fs-{}", std::process::id()));

        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();

        let markdown = r#"---
atlas:
  id: goal:ship-source-of-truth
  kind: goal
  title: Source-of-truth roadmap execution
  active_for:
    - roadmap:local-first
  claims:
    - support_tier:supported-claims
  defines:
    - proposal:source-truth-proposal
  requires:
    - spec:source-truth-spec
  decides:
    - policy_ledger:release-policy
  implements:
    - adr:stable-ids
  governs:
    - support_tier:supported-claims
  closes:
    - closeout:release-1
  supersedes:
    - plan:old-release-plan
---
# Source-of-Truth Execution
"#;

        let md_path = root.join("docs/roadmap-truth.md");
        std::fs::write(&md_path, markdown).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_markdown_file(&repo_root, Utf8Path::new("docs/roadmap-truth.md"));

        assert!(batch.diagnostics.is_empty(), "expected no diagnostics");
        assert_eq!(batch.nodes.len(), 1);
        assert_eq!(batch.nodes[0].id.as_str(), "goal:ship-source-of-truth");
        assert_eq!(batch.nodes[0].kind, NodeKind::Goal);

        let edge_kinds: Vec<_> = batch.edges.iter().map(|edge| edge.kind).collect();
        assert!(edge_kinds.contains(&EdgeKind::ActiveFor));
        assert!(edge_kinds.contains(&EdgeKind::Claims));
        assert!(edge_kinds.contains(&EdgeKind::Defines));
        assert!(edge_kinds.contains(&EdgeKind::Requires));
        assert!(edge_kinds.contains(&EdgeKind::Decides));
        assert!(edge_kinds.contains(&EdgeKind::Implements));
        assert!(edge_kinds.contains(&EdgeKind::Governs));
        assert!(edge_kinds.contains(&EdgeKind::Closes));
        assert!(edge_kinds.contains(&EdgeKind::Supersedes));
        assert_eq!(batch.edges.len(), 9);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_active_goal_manifest() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-active-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".codex/goals")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();

        let manifest = r#"
goal = "goal:operationalize-atlas"
plan = "plan:release-1"
proposal = "proposal:release-plan"
spec = "spec:release-spec"
ready_work_items = ["scen:plan-release", "scen:docs-check"]
"#;

        std::fs::write(root.join(".codex/goals/active.toml"), manifest).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/engine\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/engine")).unwrap();
        std::fs::write(
            root.join("crates/engine/Cargo.toml"),
            "[package]\nname = \"engine\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let request = DiscoverRequest {
            repo_root: Utf8PathBuf::from_path_buf(root.clone()).unwrap(),
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery.discover(&request).expect("discovery succeeds");

        let active_goal = discovered.config.active_goal.expect("active goal loaded");
        assert_eq!(
            active_goal.goal,
            Some("goal:operationalize-atlas".to_string())
        );
        assert_eq!(active_goal.plan, Some("plan:release-1".to_string()));
        assert_eq!(
            active_goal.ready_work_items,
            vec!["scen:plan-release", "scen:docs-check"]
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
