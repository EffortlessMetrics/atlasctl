#![forbid(unsafe_code)]

use atlasctl_codes::DiagnosticCode;
use atlasctl_ports::{DiscoverRequest, DiscoveryError, DiscoveryPort};
use atlasctl_types::{
    AtlasConfig, AtlasDiagnostic, AtlasEdge, AtlasId, AtlasNode, DiscoveredRepo, EdgeKind,
    NodeKind, PathSelector, Provenance, RepoDescriptor,
};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::MetadataCommand;
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

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let files = collect_candidate_files(&repo_root, &config.discovery.roots, &config.discovery.ignore);
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

fn load_config(repo_root: &Utf8Path, explicit: Option<&Utf8PathBuf>) -> (AtlasConfig, Vec<AtlasDiagnostic>) {
    let mut diagnostics = Vec::new();
    let config_path = explicit
        .cloned()
        .unwrap_or_else(|| repo_root.join("atlas.toml"));

    if !config_path.exists() {
        return (AtlasConfig::default(), diagnostics);
    }

    let rel = relative_path(repo_root, &config_path).unwrap_or_else(|| Utf8PathBuf::from("atlas.toml"));
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
            if let Some(rel) = relative_path(repo_root, &abs_root) {
                if !is_ignored(&rel, ignored) {
                    files.insert(rel);
                }
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
                format!("failed to parse fragment `{}`: {err}", rel_path),
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
        paths: if raw.paths.is_empty() {
            vec![rel_path.as_str().to_string()]
        } else {
            raw.paths
        },
        attrs: raw.attrs,
    };

    match parse_node(node, rel_path, Some(id.clone())) {
        Ok(node) => batch.nodes.push(node),
        Err(diagnostic) => batch.diagnostics.push(diagnostic),
    }

    for target in raw.explains {
        push_frontmatter_edge(
            &mut batch,
            &id,
            EdgeKind::Explains,
            &target,
            rel_path,
        );
    }
    for target in raw.proves {
        push_frontmatter_edge(&mut batch, &id, EdgeKind::Proves, &target, rel_path);
    }
    for target in raw.uses_fixture {
        push_frontmatter_edge(
            &mut batch,
            &id,
            EdgeKind::UsesFixture,
            &target,
            rel_path,
        );
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

    batch
}

fn push_frontmatter_edge(
    batch: &mut DiscoveryBatch,
    from: &str,
    kind: EdgeKind,
    to: &str,
    rel_path: &Utf8Path,
) {
    match (AtlasId::parse(from.to_string()), AtlasId::parse(to.to_string())) {
        (Ok(from), Ok(to)) => batch.edges.push(AtlasEdge {
            from,
            kind,
            to,
            provenance: Provenance::new(rel_path.to_path_buf()),
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

    let mut provenance = Provenance::new(rel_path.to_path_buf());
    provenance.fragment = fragment;

    Ok(AtlasNode {
        id,
        kind,
        title: raw.title,
        summary: raw.summary,
        paths: raw.paths.into_iter().map(PathSelector::new).collect(),
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

    let mut provenance = Provenance::new(rel_path.to_path_buf());
    provenance.fragment = fragment;

    Ok(AtlasEdge {
        from,
        kind,
        to,
        provenance,
    })
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

        let manifest_dir = package
            .manifest_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| package.manifest_path.clone());

        let rel_dir = relative_path(repo_root, &manifest_dir)
            .unwrap_or_else(|| Utf8PathBuf::from(""));

        let mut attrs = BTreeMap::new();
        attrs.insert("manifest_path".to_string(), Value::String(package.manifest_path.to_string()));
        attrs.insert("version".to_string(), Value::String(package.version.to_string()));

        batch.nodes.push(AtlasNode {
            id: crate_id,
            kind: NodeKind::Crate,
            title: package.name,
            summary: None,
            paths: vec![PathSelector::new(rel_dir.as_str())],
            attrs,
            provenance: Provenance::new(Utf8PathBuf::from("Cargo.toml")),
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
        path: path.to_path_buf(),
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
}
