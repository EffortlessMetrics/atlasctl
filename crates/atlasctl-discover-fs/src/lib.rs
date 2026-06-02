#![forbid(unsafe_code)]

use atlasctl_app::{
    DiffError, DiffPort, DiscoverRequest, DiscoveryError, DiscoveryPort, OwnersError, OwnersPort,
};
use atlasctl_types::{
    ActiveGoalConfig, AtlasConfig, AtlasDiagnostic, AtlasEdge, AtlasId, AtlasNode, DiagnosticCode,
    DiscoveredRepo, EdgeKind, NodeKind, PathSelector, Provenance, RepoDescriptor, RepoRelativePath,
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
                FileKind::PolicyToml => {
                    let parsed = parse_policy_file(&repo_root, &rel_path);
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
                owners: Vec::new(),
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
    PolicyToml,
    Other,
}

fn classify_path(path: &Utf8Path) -> FileKind {
    let name = path.file_name().unwrap_or_default();
    if name.ends_with(".atlas.yaml") || name.ends_with(".atlas.yml") {
        FileKind::Fragment
    } else if name.ends_with(".toml") && is_under_directory(path, "policy") {
        FileKind::PolicyToml
    } else if name.ends_with(".md") {
        FileKind::Markdown
    } else {
        FileKind::Other
    }
}

fn is_under_directory(path: &Utf8Path, root: &str) -> bool {
    path.components()
        .next()
        .is_some_and(|component| component.as_str() == root)
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
    #[serde(default)]
    relation: Option<String>,
    #[serde(default)]
    kind: Option<String>,
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

#[derive(Debug, Deserialize)]
struct PolicyFrontmatterEnvelope {
    atlas: Option<PolicyFrontmatter>,
}

#[derive(Debug, Deserialize, Default)]
struct PolicyFrontmatter {
    id: Option<String>,
    kind: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    governs: Vec<String>,
    #[serde(default)]
    proves: Vec<String>,
    #[serde(default)]
    surfaces: Vec<String>,
}

fn parse_policy_file(repo_root: &Utf8Path, rel_path: &Utf8Path) -> DiscoveryBatch {
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

    let envelope = match toml::from_str::<PolicyFrontmatterEnvelope>(&contents) {
        Ok(envelope) => envelope,
        Err(err) => {
            batch.diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::MalformedFragment,
                format!("failed to parse policy file `{}`: {err}", rel_path),
                None,
                Some(location(rel_path)),
            ));
            return batch;
        }
    };

    let Some(raw) = envelope.atlas else {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::PolicyFileLegacyNoAtlas,
            format!(
                "policy file `{}` does not define an `atlas` section and was skipped",
                rel_path
            ),
            None,
            Some(location(rel_path)),
        ));
        return batch;
    };

    let Some(id) = raw.id else {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!("policy file `{}` is missing `atlas.id`", rel_path),
            None,
            Some(location(rel_path)),
        ));
        return batch;
    };

    let Some(kind) = raw.kind else {
        batch.diagnostics.push(AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!("policy file `{}` is missing `atlas.kind`", rel_path),
            None,
            Some(location(rel_path)),
        ));
        return batch;
    };

    let mut attrs = BTreeMap::new();
    let surfaces = raw.surfaces.clone();
    if !surfaces.is_empty() {
        attrs.insert(
            "surfaces".to_string(),
            Value::Array(surfaces.iter().cloned().map(Value::String).collect()),
        );
    }

    let node = RawNode {
        id: id.clone(),
        kind,
        title: raw.title.unwrap_or_else(|| id.clone()),
        summary: raw.summary,
        paths: Vec::new(),
        owns: vec![rel_path.as_str().to_string()],
        touches: surfaces,
        attrs,
    };

    match parse_node(node, rel_path, Some(id.clone())) {
        Ok(node) => batch.nodes.push(node),
        Err(diagnostic) => {
            batch.diagnostics.push(diagnostic);
            return batch;
        }
    };

    for target in raw.governs {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Governs, &target, rel_path);
    }
    for target in raw.proves {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Proves, &target, rel_path);
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

    let relation = raw.relation.clone().or_else(|| raw.kind.clone());

    if raw.relation.is_some() && raw.kind.is_some() && raw.relation != raw.kind {
        return Err(AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!(
                "edge in `{}` has conflicting `relation` `{}` and `kind` `{}`",
                rel_path,
                raw.relation.unwrap_or_default(),
                raw.kind.unwrap_or_default()
            ),
            Some(from.clone()),
            Some(location(rel_path)),
        ));
    }

    let relation = relation.ok_or_else(|| {
        AtlasDiagnostic::new(
            DiagnosticCode::MalformedFragment,
            format!(
                "edge in `{}` is missing `relation` (or legacy `kind`)",
                rel_path
            ),
            Some(from.clone()),
            Some(location(rel_path)),
        )
    })?;

    let kind = relation.parse::<EdgeKind>().map_err(|invalid| {
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
    fn parses_fragment_edges_with_relation_only() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-rel-only-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let content = r#"
nodes:
  - id: req:source-of-truth
    kind: requirement
    title: Source truth
  - id: req:ship-proof
    kind: requirement
    title: Ship proof
edges:
  - from: req:source-of-truth
    to: req:ship-proof
    relation: proves
"#;
        let fragment_path = root.join("atlas/relations.atlas.yaml");
        std::fs::create_dir_all(root.join("atlas")).unwrap();
        std::fs::write(&fragment_path, content).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_fragment_file(&repo_root, Utf8Path::new("atlas/relations.atlas.yaml"));

        assert!(
            batch.diagnostics.is_empty(),
            "expected relation-only edge to parse"
        );
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, EdgeKind::Proves);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_fragment_edges_with_legacy_kind_only() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-kind-only-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let content = r#"
nodes:
  - id: req:source-of-truth
    kind: requirement
    title: Source truth
  - id: req:ship-proof
    kind: requirement
    title: Ship proof
edges:
  - from: req:source-of-truth
    to: req:ship-proof
    kind: proves
"#;
        let fragment_path = root.join("atlas/legacy-kind.atlas.yaml");
        std::fs::create_dir_all(root.join("atlas")).unwrap();
        std::fs::write(&fragment_path, content).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_fragment_file(&repo_root, Utf8Path::new("atlas/legacy-kind.atlas.yaml"));

        assert!(
            batch.diagnostics.is_empty(),
            "expected legacy kind-only edge to parse"
        );
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, EdgeKind::Proves);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_fragment_edges_with_relation_and_same_kind() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-relation-same-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let content = r#"
nodes:
  - id: req:source-of-truth
    kind: requirement
    title: Source truth
  - id: req:ship-proof
    kind: requirement
    title: Ship proof
edges:
  - from: req:source-of-truth
    to: req:ship-proof
    relation: proves
    kind: proves
"#;
        let fragment_path = root.join("atlas/relation-same.atlas.yaml");
        std::fs::create_dir_all(root.join("atlas")).unwrap();
        std::fs::write(&fragment_path, content).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch =
            parse_fragment_file(&repo_root, Utf8Path::new("atlas/relation-same.atlas.yaml"));

        assert!(
            batch.diagnostics.is_empty(),
            "expected matching relation and kind to parse"
        );
        assert_eq!(batch.edges.len(), 1);
        assert_eq!(batch.edges[0].kind, EdgeKind::Proves);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_fragment_edges_with_conflicting_relation_and_kind() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-conflict-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let content = r#"
nodes:
  - id: req:source-of-truth
    kind: requirement
    title: Source truth
  - id: req:ship-proof
    kind: requirement
    title: Ship proof
edges:
  - from: req:source-of-truth
    to: req:ship-proof
    relation: proves
    kind: runs_with
"#;
        let fragment_path = root.join("atlas/relation-conflict.atlas.yaml");
        std::fs::create_dir_all(root.join("atlas")).unwrap();
        std::fs::write(&fragment_path, content).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_fragment_file(
            &repo_root,
            Utf8Path::new("atlas/relation-conflict.atlas.yaml"),
        );

        assert_eq!(batch.edges.len(), 0);
        assert_eq!(batch.diagnostics.len(), 1);
        assert_eq!(batch.diagnostics[0].code, DiagnosticCode::MalformedFragment);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_fragment_edges_with_missing_relation_and_kind() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let content = r#"
nodes:
  - id: req:source-of-truth
    kind: requirement
    title: Source truth
  - id: req:ship-proof
    kind: requirement
    title: Ship proof
edges:
  - from: req:source-of-truth
    to: req:ship-proof
"#;
        let fragment_path = root.join("atlas/relation-missing.atlas.yaml");
        std::fs::create_dir_all(root.join("atlas")).unwrap();
        std::fs::write(&fragment_path, content).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_fragment_file(
            &repo_root,
            Utf8Path::new("atlas/relation-missing.atlas.yaml"),
        );

        assert_eq!(batch.edges.len(), 0);
        assert_eq!(batch.diagnostics.len(), 1);
        assert_eq!(batch.diagnostics[0].code, DiagnosticCode::MalformedFragment);

        let _ = std::fs::remove_dir_all(root);
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
        assert!(matches!(
            classify_path(Utf8Path::new("policy/release-review.toml")),
            FileKind::PolicyToml
        ));
        assert!(matches!(
            classify_path(Utf8Path::new("policy/release/review.toml")),
            FileKind::PolicyToml
        ));
    }

    #[test]
    fn parses_policy_toml_file() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-policy-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("policy")).unwrap();

        let policy = r#"
[atlas]
id = "policy_ledger:release-review-guardrails"
kind = "policy_ledger"
title = "Release Review Guardrails"
summary = "Defines review checks for generated artifacts."
surfaces = ["docs/**/*.md", ".github/workflows/*.yml"]
governs = ["policy_ledger:review-policy-guidance"]
proves = ["cmd:policy-audit"]
"#;

        let policy_path = root.join("policy/release-review.toml");
        std::fs::write(&policy_path, policy).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_policy_file(&repo_root, Utf8Path::new("policy/release-review.toml"));

        assert!(
            batch.diagnostics.is_empty(),
            "expected policy file to parse cleanly"
        );
        assert_eq!(batch.nodes.len(), 1, "expected one policy node");
        assert_eq!(batch.edges.len(), 2, "expected governs and proves edges");

        let node = &batch.nodes[0];
        assert_eq!(node.id.as_str(), "policy_ledger:release-review-guardrails");
        assert_eq!(node.kind, NodeKind::PolicyLedger);
        assert!(
            node.attrs.contains_key("surfaces"),
            "surfaces should be preserved as node attrs"
        );
        assert!(
            node.touches
                .iter()
                .any(|pattern| pattern.pattern == "docs/**/*.md"),
            "surface should become a touches selector"
        );
        assert!(
            node.touches
                .iter()
                .any(|pattern| pattern.pattern == ".github/workflows/*.yml"),
            "surface should become a touches selector"
        );

        let governs = batch.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Governs
                && edge.to.as_str() == "policy_ledger:review-policy-guidance"
        });
        let proves = batch
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::Proves && edge.to.as_str() == "cmd:policy-audit");

        assert!(governs, "expected governs edge");
        assert!(proves, "expected proves edge");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_nested_policy_toml_file() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-policy-nested-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("policy/release/governance")).unwrap();

        let policy = r#"
  [atlas]
  id = "policy_ledger:release-governance"
  kind = "policy_ledger"
  title = "Release Governance"
  summary = "Nested governance policy metadata."
  surfaces = ["docs/**/*.md"]
  proves = ["cmd:policy-audit"]
"#;

        let policy_path = root.join("policy/release/governance/review-process.toml");
        std::fs::write(&policy_path, policy).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_policy_file(
            &repo_root,
            Utf8Path::new("policy/release/governance/review-process.toml"),
        );

        assert!(
            batch.diagnostics.is_empty(),
            "expected nested policy file to parse cleanly"
        );
        assert_eq!(batch.nodes.len(), 1, "expected one policy node");
        assert_eq!(
            batch.nodes[0].id.as_str(),
            "policy_ledger:release-governance"
        );
        assert_eq!(batch.edges.len(), 1, "expected one proves edge");
        assert!(
            batch.edges[0].kind == EdgeKind::Proves
                && batch.edges[0].to.as_str() == "cmd:policy-audit"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_legacy_policy_toml_file_as_warning() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-policy-legacy-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("policy")).unwrap();

        let policy = r#"
enabled = true
message = "legacy policy file with no atlas metadata"
"#;

        let policy_path = root.join("policy/legacy-legacy-policy.toml");
        std::fs::write(&policy_path, policy).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let batch = parse_policy_file(
            &repo_root,
            Utf8Path::new("policy/legacy-legacy-policy.toml"),
        );

        assert_eq!(
            batch.nodes.len(),
            0,
            "legacy policy files should not emit nodes"
        );
        assert_eq!(
            batch.edges.len(),
            0,
            "legacy policy files should not emit edges"
        );
        assert_eq!(
            batch.diagnostics.len(),
            1,
            "expected warning diagnostic for missing atlas section"
        );
        assert_eq!(
            batch.diagnostics[0].code,
            DiagnosticCode::PolicyFileLegacyNoAtlas,
            "expected legacy policy warning code"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discover_includes_policy_toml_via_default_roots() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-policy-discovery-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("policy")).unwrap();

        let atlas_config = r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs", "policy"]
ignore = ["target", ".git", "node_modules"]
"#;
        std::fs::write(root.join("atlas.toml"), atlas_config).unwrap();

        let policy = r#"
[atlas]
id = "policy_ledger:discovery-test"
kind = "policy_ledger"
title = "Discovery test policy"
summary = "Ensures policy files are discovered."
surfaces = ["policy/**/*.toml"]
proves = ["cmd:policy-check"]
"#;
        std::fs::write(root.join("policy/test-policy.toml"), policy).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let discovery = FsDiscovery;
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed for policy fixture");

        assert!(
            discovered
                .nodes
                .iter()
                .any(|node| node.id.as_str() == "policy_ledger:discovery-test"),
            "policy ledger nodes under policy/ should be discovered"
        );

        let _ = std::fs::remove_dir_all(root);
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
    fn parses_support_tier_claim_links() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "atlasctl-discover-fs-support-tier-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs/status")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();

        let support_tier = r#"---
atlas:
  id: support_tier:docs-support
  kind: support_tier
  title: Docs support tier
  proves:
    - cmd:docs-proof
  claims:
    - claim:docs-readme-accuracy
---
# Docs support tier
"#;

        let claim = r#"---
atlas:
  id: claim:docs-readme-accuracy
  kind: claim
  title: README keeps claims truthful
  supports:
    - support_tier:docs-support
  proves:
    - cmd:docs-proof
---
# README
"#;

        let md_path = root.join("docs/status/SUPPORT_TIERS.md");
        let claim_path = root.join("docs/README.md");

        std::fs::write(&md_path, support_tier).unwrap();
        std::fs::write(&claim_path, claim).unwrap();

        let repo_root = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
        let support_batch =
            parse_markdown_file(&repo_root, Utf8Path::new("docs/status/SUPPORT_TIERS.md"));
        let claim_batch = parse_markdown_file(&repo_root, Utf8Path::new("docs/README.md"));

        let batch_nodes: Vec<_> = support_batch
            .nodes
            .into_iter()
            .chain(claim_batch.nodes)
            .collect();
        let batch_edges: Vec<_> = support_batch
            .edges
            .into_iter()
            .chain(claim_batch.edges)
            .collect();

        assert!(
            support_batch.diagnostics.is_empty(),
            "expected no diagnostics in support tier fixture"
        );
        assert!(
            claim_batch.diagnostics.is_empty(),
            "expected no diagnostics in claim fixture"
        );
        assert_eq!(batch_nodes.len(), 2);
        assert!(batch_edges.iter().any(|edge| {
            edge.from.as_str() == "support_tier:docs-support"
                && edge.kind == EdgeKind::Claims
                && edge.to.as_str() == "claim:docs-readme-accuracy"
        }));
        assert!(batch_edges.iter().any(|edge| {
            edge.from.as_str() == "support_tier:docs-support"
                && edge.kind == EdgeKind::Proves
                && edge.to.as_str() == "cmd:docs-proof"
        }));
        assert!(batch_edges.iter().any(|edge| {
            edge.from.as_str() == "claim:docs-readme-accuracy"
                && edge.kind == EdgeKind::Supports
                && edge.to.as_str() == "support_tier:docs-support"
        }));
        assert!(batch_edges.iter().any(|edge| {
            edge.from.as_str() == "claim:docs-readme-accuracy"
                && edge.kind == EdgeKind::Proves
                && edge.to.as_str() == "cmd:docs-proof"
        }));

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
