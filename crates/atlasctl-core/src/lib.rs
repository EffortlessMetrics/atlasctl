#![forbid(unsafe_code)]

use atlasctl_codes::{DiagnosticCode, Severity};
use atlasctl_types::{
    AtlasDiagnostic, AtlasEdge, AtlasGraph, AtlasId, AtlasMetrics, AtlasNode, DiscoveredRepo,
    EdgeKind, NodeKind, NodeMatch, ProfileSettings, QueryRequest, QueryResponse,
    SourceLocation, TraceDirection, TraceEdge, TraceRequest, TraceResponse, ValidationProfile,
    ATLAS_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn compile_atlas(discovered: DiscoveredRepo, profile: ValidationProfile) -> AtlasGraph {
    let settings = discovered.config.profile_settings(profile);
    let mut diagnostics = discovered.diagnostics.clone();

    let mut unique_nodes = BTreeMap::<AtlasId, AtlasNode>::new();
    let mut sorted_nodes = discovered.nodes.clone();
    sorted_nodes.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.provenance.source.cmp(&right.provenance.source))
    });

    for node in sorted_nodes {
        if let Some(existing) = unique_nodes.get(&node.id) {
            diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::DuplicateId,
                format!(
                    "duplicate atlas id `{}` declared in both `{}` and `{}`",
                    node.id, existing.provenance.source, node.provenance.source
                ),
                Some(node.id.clone()),
                Some(node.provenance.location()),
            ));
            continue;
        }

        unique_nodes.insert(node.id.clone(), node);
    }

    let node_kind_index: BTreeMap<AtlasId, NodeKind> = unique_nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.kind))
        .collect();

    let mut valid_edges = Vec::new();
    let mut sorted_edges = discovered.edges.clone();
    sorted_edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| left.provenance.source.cmp(&right.provenance.source))
    });

    for edge in sorted_edges {
        let Some(from_kind) = node_kind_index.get(&edge.from).copied() else {
            diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::BrokenReference,
                format!(
                    "edge `{}` -> `{}` references missing source node `{}`",
                    edge.from, edge.to, edge.from
                ),
                Some(edge.from.clone()),
                Some(edge.provenance.location()),
            ));
            continue;
        };

        let Some(to_kind) = node_kind_index.get(&edge.to).copied() else {
            let code = if edge.kind == EdgeKind::RunsWith {
                DiagnosticCode::CommandReferencedButUndeclared
            } else {
                DiagnosticCode::BrokenReference
            };

            diagnostics.push(AtlasDiagnostic::new(
                code,
                format!(
                    "edge `{}` -> `{}` references missing target node `{}`",
                    edge.from, edge.to, edge.to
                ),
                Some(edge.to.clone()),
                Some(edge.provenance.location()),
            ));
            continue;
        };

        if !edge_endpoint_is_valid(edge.kind, from_kind, to_kind) {
            diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::InvalidEdgeEndpoint,
                format!(
                    "edge kind `{}` is not valid between `{}` and `{}`",
                    edge.kind, from_kind, to_kind
                ),
                Some(edge.from.clone()),
                Some(edge.provenance.location()),
            ));
            continue;
        }

        valid_edges.push(edge);
    }

    let nodes: Vec<AtlasNode> = unique_nodes.values().cloned().collect();
    validate_completeness(&nodes, &valid_edges, &settings, &mut diagnostics);
    apply_profile_escalation(&settings, &mut diagnostics);
    sort_diagnostics(&mut diagnostics);

    let metrics = AtlasMetrics {
        node_count: nodes.len(),
        edge_count: valid_edges.len(),
        diagnostic_count: diagnostics.len(),
        error_count: diagnostics
            .iter()
            .filter(|diag| diag.severity == Severity::Error)
            .count(),
        warning_count: diagnostics
            .iter()
            .filter(|diag| diag.severity == Severity::Warning)
            .count(),
    };

    AtlasGraph {
        schema_version: ATLAS_SCHEMA_VERSION,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repo: discovered.repo,
        nodes,
        edges: valid_edges,
        diagnostics,
        metrics,
    }
}

pub fn query_graph(graph: &AtlasGraph, request: &QueryRequest) -> QueryResponse {
    let needle = request.needle.trim().to_lowercase();
    let mut matches = Vec::new();

    for node in &graph.nodes {
        if let Some(kind) = request.kind {
            if node.kind != kind {
                continue;
            }
        }

        let mut score = 0_u32;
        if node.id.as_str().eq_ignore_ascii_case(&request.needle) {
            score = 100;
        } else if node.id.as_str().to_lowercase().contains(&needle) {
            score = 80;
        } else if node.title.to_lowercase().contains(&needle) {
            score = 60;
        } else if node
            .summary
            .as_ref()
            .map(|summary| summary.to_lowercase().contains(&needle))
            .unwrap_or(false)
        {
            score = 40;
        }

        if score > 0 {
            matches.push(NodeMatch {
                score,
                node: node.clone(),
            });
        }
    }

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.node.id.cmp(&right.node.id))
    });

    QueryResponse {
        needle: request.needle.clone(),
        matches,
    }
}

pub fn trace_graph(graph: &AtlasGraph, request: &TraceRequest) -> Option<TraceResponse> {
    let root = graph
        .nodes
        .iter()
        .find(|node| node.id == request.start)
        .cloned()?;

    let node_map: BTreeMap<AtlasId, AtlasNode> = graph
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id.clone(), node))
        .collect();

    let mut outgoing = BTreeMap::<AtlasId, Vec<AtlasEdge>>::new();
    let mut incoming = BTreeMap::<AtlasId, Vec<AtlasEdge>>::new();

    for edge in &graph.edges {
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.clone());
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.clone());
    }

    for edges in outgoing.values_mut() {
        edges.sort_by(|left, right| {
            left.to
                .cmp(&right.to)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.from.cmp(&right.from))
        });
    }

    for edges in incoming.values_mut() {
        edges.sort_by(|left, right| {
            left.from
                .cmp(&right.from)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.to.cmp(&right.to))
        });
    }

    let mut queue = VecDeque::new();
    let mut visited = BTreeSet::new();
    let mut traced_edges = Vec::new();
    let mut traced_nodes = Vec::new();

    visited.insert(root.id.clone());
    queue.push_back((root.id.clone(), 0_usize));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= request.max_depth {
            continue;
        }

        let candidate_edges = match request.direction {
            TraceDirection::Outgoing => outgoing.get(&current).cloned().unwrap_or_default(),
            TraceDirection::Incoming => incoming.get(&current).cloned().unwrap_or_default(),
            TraceDirection::Both => {
                let mut both = outgoing.get(&current).cloned().unwrap_or_default();
                both.extend(incoming.get(&current).cloned().unwrap_or_default());
                both.sort_by(|left, right| {
                    left.from
                        .cmp(&right.from)
                        .then_with(|| left.to.cmp(&right.to))
                        .then_with(|| left.kind.cmp(&right.kind))
                });
                both
            }
        };

        for edge in candidate_edges {
            let next = match request.direction {
                TraceDirection::Incoming => edge.from.clone(),
                TraceDirection::Outgoing => edge.to.clone(),
                TraceDirection::Both => {
                    if edge.from == current {
                        edge.to.clone()
                    } else {
                        edge.from.clone()
                    }
                }
            };

            traced_edges.push(TraceEdge {
                depth: depth + 1,
                edge: edge.clone(),
            });

            if visited.insert(next.clone()) {
                if let Some(node) = node_map.get(&next) {
                    traced_nodes.push(node.clone());
                }
                queue.push_back((next, depth + 1));
            }
        }
    }

    Some(TraceResponse {
        root,
        nodes: traced_nodes,
        edges: traced_edges,
    })
}

fn validate_completeness(
    nodes: &[AtlasNode],
    edges: &[AtlasEdge],
    settings: &ProfileSettings,
    diagnostics: &mut Vec<AtlasDiagnostic>,
) {
    let mut outgoing = BTreeMap::<AtlasId, Vec<&AtlasEdge>>::new();
    let mut incoming = BTreeMap::<AtlasId, Vec<&AtlasEdge>>::new();

    for edge in edges {
        outgoing.entry(edge.from.clone()).or_default().push(edge);
        incoming.entry(edge.to.clone()).or_default().push(edge);
    }

    for node in nodes {
        match node.kind {
            NodeKind::Scenario => {
                if settings.require_scenario_command {
                    let has_command = outgoing
                        .get(&node.id)
                        .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::RunsWith))
                        .unwrap_or(false);

                    if !has_command {
                        diagnostics.push(AtlasDiagnostic::new(
                            DiagnosticCode::ScenarioMissingCommand,
                            format!("scenario `{}` does not declare a proving command", node.id),
                            Some(node.id.clone()),
                            Some(node.provenance.location()),
                        ));
                    }
                }

                if settings.require_scenario_crate {
                    let has_crate = outgoing
                        .get(&node.id)
                        .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::Exercises))
                        .unwrap_or(false);

                    if !has_crate {
                        diagnostics.push(AtlasDiagnostic::new(
                            DiagnosticCode::ScenarioMissingCrate,
                            format!("scenario `{}` does not declare an exercised crate", node.id),
                            Some(node.id.clone()),
                            Some(node.provenance.location()),
                        ));
                    }
                }
            }
            NodeKind::Artifact => {
                if settings.require_artifact_producer {
                    let has_producer = incoming
                        .get(&node.id)
                        .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::Emits))
                        .unwrap_or(false);

                    if !has_producer {
                        diagnostics.push(AtlasDiagnostic::new(
                            DiagnosticCode::ArtifactMissingProducer,
                            format!("artifact `{}` does not have an emitting edge", node.id),
                            Some(node.id.clone()),
                            Some(node.provenance.location()),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

fn apply_profile_escalation(settings: &ProfileSettings, diagnostics: &mut [AtlasDiagnostic]) {
    if settings.warnings_as_errors {
        for diagnostic in diagnostics {
            if diagnostic.severity == Severity::Warning {
                diagnostic.severity = Severity::Error;
            }
        }
    }
}

fn sort_diagnostics(diagnostics: &mut Vec<AtlasDiagnostic>) {
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| compare_locations(left.location.as_ref(), right.location.as_ref()))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn compare_locations(left: Option<&SourceLocation>, right: Option<&SourceLocation>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left
            .path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn edge_endpoint_is_valid(kind: EdgeKind, from: NodeKind, to: NodeKind) -> bool {
    match kind {
        EdgeKind::UsesFixture => from == NodeKind::Scenario && to == NodeKind::Fixture,
        EdgeKind::RunsWith => from == NodeKind::Scenario && to == NodeKind::Command,
        EdgeKind::Exercises => from == NodeKind::Scenario && to == NodeKind::Crate,
        EdgeKind::Emits => to == NodeKind::Artifact,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlasctl_types::{
        AtlasConfig, AtlasEdge, AtlasId, AtlasNode, DiscoveredRepo, NodeKind, PathSelector,
        Provenance, RepoDescriptor,
    };
    use std::collections::BTreeMap;

    fn node(id: &str, kind: NodeKind) -> AtlasNode {
        AtlasNode {
            id: AtlasId::parse(id).expect("valid id"),
            kind,
            title: id.to_string(),
            summary: None,
            paths: vec![PathSelector::new("src/lib.rs")],
            attrs: BTreeMap::new(),
            provenance: Provenance::new("atlas/example.atlas.yaml".into()),
        }
    }

    fn edge(from: &str, kind: EdgeKind, to: &str) -> AtlasEdge {
        AtlasEdge {
            from: AtlasId::parse(from).expect("valid from"),
            kind,
            to: AtlasId::parse(to).expect("valid to"),
            provenance: Provenance::new("atlas/example.atlas.yaml".into()),
        }
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![node("scen:one", NodeKind::Scenario), node("scen:one", NodeKind::Scenario)],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        assert!(graph
            .diagnostics
            .iter()
            .any(|diag| diag.code == DiagnosticCode::DuplicateId));
    }

    #[test]
    fn scenario_missing_command_is_reported() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![node("scen:one", NodeKind::Scenario), node("crate:engine", NodeKind::Crate)],
            edges: vec![edge("scen:one", EdgeKind::Exercises, "crate:engine")],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        assert!(graph
            .diagnostics
            .iter()
            .any(|diag| diag.code == DiagnosticCode::ScenarioMissingCommand));
    }

    #[test]
    fn query_finds_exact_id_first() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:one", NodeKind::Scenario),
                node("req:one", NodeKind::Requirement),
                node("crate:engine", NodeKind::Crate),
                node("cmd:ci-fast", NodeKind::Command),
            ],
            edges: vec![
                edge("scen:one", EdgeKind::Exercises, "crate:engine"),
                edge("scen:one", EdgeKind::RunsWith, "cmd:ci-fast"),
            ],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "scen:one".to_string(),
                kind: None,
            },
        );

        assert_eq!(response.matches.first().map(|entry| entry.score), Some(100));
    }
}
