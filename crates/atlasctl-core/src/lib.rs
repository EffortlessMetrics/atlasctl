#![forbid(unsafe_code)]

use atlasctl_codes::{DiagnosticCode, Severity};
use atlasctl_types::{
    ATLAS_SCHEMA_VERSION, AtlasDiagnostic, AtlasEdge, AtlasGraph, AtlasId, AtlasMetrics, AtlasNode,
    DiscoveredRepo, EdgeKind, ImpactHit, ImpactRequest, ImpactResponse, NodeKind, NodeMatch,
    ProfileSettings, QueryRequest, QueryResponse, SourceLocation, TraceDirection, TraceEdge,
    TraceRequest, TraceResponse, ValidationProfile, WhyRequest, WhyResponse, WhyStep, WhySubject,
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
        if let Some(kind) = request.kind
            && node.kind != kind
        {
            continue;
        }
        let mut score = 0;
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

pub fn why_graph(graph: &AtlasGraph, request: &WhyRequest) -> Option<WhyResponse> {
    let root = match &request.subject {
        WhySubject::Id(id) => graph.nodes.iter().find(|n| n.id == *id)?,
        WhySubject::Path(path) => {
            // Find nodes that have a selector matching this path
            graph.nodes.iter().find(|n| {
                n.all_paths().any(|p| {
                    let pattern = p.pattern.replace('\\', "/");
                    let glob: Option<globset::GlobMatcher> = globset::Glob::new(&pattern)
                        .ok()
                        .and_then(|g| g.compile_matcher().into());
                    if let Some(glob) = glob {
                        glob.is_match(path.as_str())
                    } else {
                        false
                    }
                })
            })?
        }
    };

    let mut chain = Vec::new();
    let mut visited = BTreeSet::new();
    visited.insert(root.id.clone());

    // Look for immediate relationships that "explain" this node
    for edge in &graph.edges {
        if edge.to == root.id {
            // Incoming edges
            match edge.kind {
                EdgeKind::Explains
                | EdgeKind::Proves
                | EdgeKind::Exercises
                | EdgeKind::RunsWith
                | EdgeKind::Documents
                | EdgeKind::BelongsTo
                | EdgeKind::Supports => {
                    if let Some(node) = graph.nodes.iter().find(|n| n.id == edge.from)
                        && visited.insert(node.id.clone())
                    {
                        chain.push(WhyStep {
                            node: node.clone(),
                            relationship: edge.kind,
                            direction: TraceDirection::Incoming,
                        });
                    }
                }
                _ => {}
            }
        } else if edge.from == root.id {
            // Outgoing edges
            match edge.kind {
                EdgeKind::Emits | EdgeKind::Exercises | EdgeKind::RunsWith => {
                    if let Some(node) = graph.nodes.iter().find(|n| n.id == edge.to)
                        && visited.insert(node.id.clone())
                    {
                        chain.push(WhyStep {
                            node: node.clone(),
                            relationship: edge.kind,
                            direction: TraceDirection::Outgoing,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Some(WhyResponse {
        root: root.clone(),
        chain,
    })
}

pub fn impacted_graph(graph: &AtlasGraph, request: &ImpactRequest) -> ImpactResponse {
    let mut impacted = BTreeMap::<AtlasId, ImpactHit>::new();
    let mut uncovered = Vec::new();

    for changed in &request.paths {
        let mut found_any = false;
        for node in &graph.nodes {
            for selector in node.all_paths() {
                let pattern = selector.pattern.replace('\\', "/");
                let glob: Option<globset::GlobMatcher> = globset::Glob::new(&pattern)
                    .ok()
                    .and_then(|g| g.compile_matcher().into());
                if let Some(glob) = glob
                    && glob.is_match(changed.path.as_str())
                {
                    found_any = true;

                    let mut hit_owners = Vec::new();
                    if let Some(owners) = request.owners.get(&changed.path) {
                        hit_owners.extend(owners.clone());
                    }

                    let entry = impacted
                        .entry(node.id.clone())
                        .or_insert_with(|| ImpactHit {
                            node: node.clone(),
                            reason: format!("matches changed path `{}`", changed.path),
                            owners: Vec::new(),
                        });

                    // Merge owners
                    for o in hit_owners {
                        if !entry.owners.contains(&o) {
                            entry.owners.push(o);
                        }
                    }
                }
            }
        }
        if !found_any {
            uncovered.push(changed.clone());
        }
    }

    // Graph expansion: 1 step from direct hits
    let direct_hits: Vec<_> = impacted.keys().cloned().collect();
    for hit_id in direct_hits {
        for edge in &graph.edges {
            if edge.from == hit_id {
                // Outgoing
                if let Some(to_node) = graph.nodes.iter().find(|n| n.id == edge.to) {
                    let from_owners = impacted
                        .get(&hit_id)
                        .map(|h| h.owners.clone())
                        .unwrap_or_default();
                    let entry = impacted
                        .entry(to_node.id.clone())
                        .or_insert_with(|| ImpactHit {
                            node: to_node.clone(),
                            reason: format!("related to `{}` via `{}`", hit_id, edge.kind),
                            owners: Vec::new(),
                        });

                    // Propagate owners
                    for o in from_owners {
                        if !entry.owners.contains(&o) {
                            entry.owners.push(o);
                        }
                    }
                }
            } else if edge.to == hit_id {
                // Incoming
                if let Some(from_node) = graph.nodes.iter().find(|n| n.id == edge.from) {
                    let to_owners = impacted
                        .get(&hit_id)
                        .map(|h| h.owners.clone())
                        .unwrap_or_default();
                    let entry = impacted
                        .entry(from_node.id.clone())
                        .or_insert_with(|| ImpactHit {
                            node: from_node.clone(),
                            reason: format!("relates to `{}` via `{}`", hit_id, edge.kind),
                            owners: Vec::new(),
                        });

                    // Propagate owners
                    for o in to_owners {
                        if !entry.owners.contains(&o) {
                            entry.owners.push(o);
                        }
                    }
                }
            }
        }
    }

    let mut impacted_list: Vec<_> = impacted.into_values().collect();
    impacted_list.sort_by(|a, b| a.node.id.cmp(&b.node.id));

    ImpactResponse {
        impacted: impacted_list,
        uncovered,
    }
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
        let has_any_edge = outgoing.contains_key(&node.id) || incoming.contains_key(&node.id);

        // Infra nodes like crates and operational commands are allowed to be loosely coupled
        if !has_any_edge
            && node.kind.role() != atlasctl_types::NodeRole::Infra
            && node.kind != NodeKind::Command
        {
            diagnostics.push(AtlasDiagnostic::new(
                DiagnosticCode::OrphanNode,
                format!("node `{}` has no relationships in the graph", node.id),
                Some(node.id.clone()),
                Some(node.provenance.location()),
            ));
        }

        match node.kind {
            NodeKind::Command => {
                let is_used = incoming
                    .get(&node.id)
                    .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::RunsWith))
                    .unwrap_or(false);

                if !is_used {
                    diagnostics.push(AtlasDiagnostic::new(
                        DiagnosticCode::StaleCommand,
                        format!("command `{}` is not used by any scenario", node.id),
                        Some(node.id.clone()),
                        Some(node.provenance.location()),
                    ));
                }
            }
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
            NodeKind::Requirement => {
                if settings.require_requirement_proof {
                    let is_proven = incoming
                        .get(&node.id)
                        .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::Proves))
                        .unwrap_or(false);

                    if !is_proven {
                        diagnostics.push(AtlasDiagnostic::new(
                            DiagnosticCode::RequirementNotProven,
                            format!("requirement `{}` is not proven by any scenario", node.id),
                            Some(node.id.clone()),
                            Some(node.provenance.location()),
                        ));
                    }
                }
            }
            NodeKind::Crate => {
                if settings.require_crate_scenario {
                    let is_exercised = incoming
                        .get(&node.id)
                        .map(|edges| edges.iter().any(|edge| edge.kind == EdgeKind::Exercises))
                        .unwrap_or(false);

                    if !is_exercised {
                        diagnostics.push(AtlasDiagnostic::new(
                            DiagnosticCode::UncoveredCrate,
                            format!("crate `{}` is not exercised by any scenario", node.id),
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

fn sort_diagnostics(diagnostics: &mut [AtlasDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| compare_locations(left.location.as_ref(), right.location.as_ref()))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn compare_locations(
    left: Option<&SourceLocation>,
    right: Option<&SourceLocation>,
) -> std::cmp::Ordering {
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
    use atlasctl_ports::DiscoveryPort;
    use atlasctl_types::{
        AtlasConfig, AtlasEdge, AtlasId, AtlasNode, DiscoveredRepo, NodeKind, PathSelector,
        Provenance, RepoDescriptor, RepoRelativePath,
    };
    use proptest::proptest;
    use std::collections::BTreeMap;

    fn node(id: &str, kind: NodeKind) -> AtlasNode {
        AtlasNode {
            id: AtlasId::parse(id).expect("valid id"),
            role: kind.role(),
            kind,
            title: id.to_string(),
            summary: None,
            owns: vec![PathSelector::new("src/lib.rs")],
            touches: Vec::new(),
            attrs: BTreeMap::new(),
            provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
        }
    }

    fn edge(from: &str, kind: EdgeKind, to: &str) -> AtlasEdge {
        AtlasEdge {
            from: AtlasId::parse(from).expect("valid from"),
            kind,
            to: AtlasId::parse(to).expect("valid to"),
            provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
        }
    }

    #[test]
    fn duplicate_ids_are_reported() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:one", NodeKind::Scenario),
                node("scen:one", NodeKind::Scenario),
            ],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        assert!(
            graph
                .diagnostics
                .iter()
                .any(|diag| diag.code == DiagnosticCode::DuplicateId)
        );
    }

    #[test]
    fn scenario_missing_command_is_reported() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:one", NodeKind::Scenario),
                node("crate:engine", NodeKind::Crate),
            ],
            edges: vec![edge("scen:one", EdgeKind::Exercises, "crate:engine")],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        assert!(
            graph
                .diagnostics
                .iter()
                .any(|diag| diag.code == DiagnosticCode::ScenarioMissingCommand)
        );
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

    // ============================================================================
    // BDD/SCENARIO TESTS - End-to-end workflow testing with fixture repos
    // ============================================================================

    /// SCENARIO: Building a complete atlas from a valid minimal fixture repo
    ///
    /// GIVEN a valid minimal repository with atlas metadata
    /// WHEN the atlas is compiled with the default validation profile
    /// THEN the graph should contain all expected nodes and edges
    /// AND all diagnostics should be empty (no errors or warnings)
    #[test]
    fn scenario_build_complete_atlas_from_valid_minimal_fixture() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/valid-minimal");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        let graph = compile_atlas(discovered, ValidationProfile::Default);

        // Verify the graph structure
        assert_eq!(graph.repo.name, "valid-minimal");
        assert_eq!(graph.metrics.node_count, 6); // Includes ADR from markdown
        assert_eq!(graph.metrics.edge_count, 5); // Includes ADR explains edge

        // Verify all expected nodes are present
        let node_ids: Vec<_> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(node_ids.contains(&"req:example"));
        assert!(node_ids.contains(&"scen:example-build"));
        assert!(node_ids.contains(&"cmd:ci-fast"));
        assert!(node_ids.contains(&"artifact:example-report"));
        assert!(node_ids.contains(&"crate:engine"));
        assert!(node_ids.contains(&"adr:0001-example"));

        // Verify all expected edges are present
        let proves_edge = graph.edges.iter().find(|e| e.kind == EdgeKind::Proves);
        assert!(proves_edge.is_some());

        // Verify no diagnostics (clean build)
        assert_eq!(graph.metrics.diagnostic_count, 0);
        assert_eq!(graph.metrics.error_count, 0);
        assert_eq!(graph.metrics.warning_count, 0);
    }

    /// SCENARIO: Detecting broken references in a fixture repo
    ///
    /// GIVEN a repository with an edge referencing a non-existent node
    /// WHEN the atlas is compiled
    /// THEN a BrokenReference diagnostic should be emitted
    /// AND the invalid edge should be excluded from the graph
    #[test]
    fn scenario_detect_broken_references_from_broken_link_fixture() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/broken-link");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        let graph = compile_atlas(discovered, ValidationProfile::Default);

        // Verify broken reference diagnostic is present
        // The missing command is reported as CommandReferencedButUndeclared
        let broken_ref_diagnostics: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::CommandReferencedButUndeclared)
            .collect();

        assert!(
            !broken_ref_diagnostics.is_empty(),
            "Should have broken reference diagnostics"
        );
        assert!(graph.metrics.error_count >= 1);

        // Verify the invalid edge is excluded
        let runs_with_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::RunsWith)
            .collect();
        assert!(runs_with_edges.is_empty(), "Broken edge should be excluded");
    }

    /// SCENARIO: Detecting duplicate IDs in a fixture repo
    ///
    /// GIVEN a repository with two nodes sharing the same ID
    /// WHEN the atlas is compiled
    /// THEN a DuplicateId diagnostic should be emitted
    /// AND only the first node should be included in the graph
    #[test]
    fn scenario_detect_duplicate_ids_from_duplicate_id_fixture() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/duplicate-id");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        let graph = compile_atlas(discovered, ValidationProfile::Default);

        // Verify duplicate ID diagnostic is present
        let duplicate_diagnostics: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::DuplicateId)
            .collect();

        assert!(
            !duplicate_diagnostics.is_empty(),
            "Should have duplicate ID diagnostics"
        );
        assert_eq!(graph.metrics.error_count, 1);

        // Verify only one node with the duplicate ID exists
        let duplicate_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.id.as_str() == "req:example")
            .collect();
        assert_eq!(
            duplicate_nodes.len(),
            1,
            "Only first duplicate should be kept"
        );
    }

    /// SCENARIO: Detecting orphan scenarios in a fixture repo
    ///
    /// GIVEN a repository with a scenario that has no required edges
    /// WHEN the atlas is compiled with the default profile
    /// THEN diagnostics should be emitted for missing command and crate edges
    #[test]
    fn scenario_detect_orphan_scenarios_from_orphan_scenario_fixture() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/orphan-scenario");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        let graph = compile_atlas(discovered, ValidationProfile::Default);

        // Verify scenario missing command diagnostic is present
        let missing_command_diagnostics: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::ScenarioMissingCommand)
            .collect();

        assert!(
            !missing_command_diagnostics.is_empty(),
            "Should have missing command diagnostics"
        );

        // Verify scenario missing crate diagnostic is present
        let missing_crate_diagnostics: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::ScenarioMissingCrate)
            .collect();

        assert!(
            !missing_crate_diagnostics.is_empty(),
            "Should have missing crate diagnostics"
        );

        assert!(graph.metrics.error_count >= 2);
    }

    /// SCENARIO: Touch-path overlap is allowed across separate nodes
    ///
    /// GIVEN two scenario nodes that both touch the same path
    /// WHEN the atlas is compiled
    /// THEN there should be no duplicate_ownership diagnostics
    #[test]
    fn scenario_overlapping_touches_are_allowed() {
        let mut node_one = node("scen:touch-one", NodeKind::Scenario);
        node_one.owns.clear();
        node_one.touches = vec![PathSelector::new("crates/engine/src/lib.rs")];

        let mut node_two = node("scen:touch-two", NodeKind::Scenario);
        node_two.owns.clear();
        node_two.touches = vec![PathSelector::new("crates/engine/src/lib.rs")];

        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![node_one, node_two, node("cmd:test", NodeKind::Command)],
            edges: vec![
                edge("scen:touch-one", EdgeKind::RunsWith, "cmd:test"),
                edge("scen:touch-two", EdgeKind::RunsWith, "cmd:test"),
            ],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);

        let has_duplicate = graph
            .diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::DuplicateOwnership);

        assert!(
            !has_duplicate,
            "touch-overlapping nodes should not trigger duplicate ownership"
        );
    }

    /// SCENARIO: Infra nodes are exempt from behavior/proof ownership checks
    ///
    /// GIVEN a crate node with no outgoing/incoming edges
    /// WHEN the atlas is compiled
    /// THEN no behavior/proof-only completeness diagnostics should apply
    #[test]
    fn scenario_infra_node_without_graph_connections_is_allowed() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![AtlasNode {
                id: AtlasId::parse("crate:engine").expect("valid atlas id"),
                kind: NodeKind::Crate,
                role: NodeKind::Crate.role(),
                title: "engine".to_string(),
                summary: None,
                owns: vec![PathSelector::new("crates/engine/src/lib.rs")],
                touches: Vec::new(),
                attrs: BTreeMap::new(),
                provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
            }],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let infra_errors: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.code,
                    DiagnosticCode::ScenarioMissingCommand | DiagnosticCode::ScenarioMissingCrate
                )
            })
            .collect();

        assert!(
            infra_errors.is_empty(),
            "infra nodes should not emit behavior/proof diagnostics"
        );
    }

    /// SCENARIO: Querying the atlas with different search terms
    ///
    /// GIVEN a compiled atlas from a valid minimal fixture
    /// WHEN querying with various search terms
    /// THEN results should be returned with appropriate scores
    /// AND exact ID matches should have the highest score
    #[test]
    fn scenario_query_with_different_search_terms() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/valid-minimal");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");
        let graph = compile_atlas(discovered, ValidationProfile::Default);

        // Query 1: Exact ID match should score 100
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "scen:example-build".to_string(),
                kind: None,
            },
        );
        assert_eq!(response.needle, "scen:example-build");
        assert!(!response.matches.is_empty());
        assert_eq!(response.matches.first().map(|m| m.score), Some(100));

        // Query 2: Partial ID match should score 80
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "example".to_string(),
                kind: None,
            },
        );
        assert_eq!(response.needle, "example");
        assert!(!response.matches.is_empty());
        assert!(response.matches.iter().any(|m| m.score == 80));

        // Query 3: Partial ID match should score 80 (takes precedence over title match)
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "build".to_string(),
                kind: None,
            },
        );
        assert_eq!(response.needle, "build");
        assert!(!response.matches.is_empty());
        // "scen:example-build" ID contains "build", so scores 80
        assert!(
            response.matches.iter().any(|m| m.score == 80),
            "Expected a partial ID match with score 80"
        );

        // Query 4: Filter by node kind
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "".to_string(),
                kind: Some(NodeKind::Scenario),
            },
        );
        assert_eq!(response.needle, "");
        assert!(
            response
                .matches
                .iter()
                .all(|m| m.node.kind == NodeKind::Scenario)
        );
    }

    /// SCENARIO: Tracing the atlas with different directions
    ///
    /// GIVEN a compiled atlas from a valid minimal fixture
    /// WHEN tracing from a scenario node with different directions
    /// THEN the trace should return appropriate nodes and edges
    /// AND depth should be respected
    #[test]
    fn scenario_trace_with_different_directions() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/valid-minimal");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");
        let graph = compile_atlas(discovered, ValidationProfile::Default);

        let start_id = AtlasId::parse("scen:example-build").expect("valid id");

        // Trace 1: Outgoing edges (what the scenario exercises/emits)
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Outgoing,
                max_depth: 10,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.root.id.as_str(), "scen:example-build");
        assert!(!response.edges.is_empty());
        assert!(response.edges.iter().all(|e| e.edge.from == start_id));

        // Trace 2: Incoming edges (what references the scenario)
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Incoming,
                max_depth: 10,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.root.id.as_str(), "scen:example-build");
        // No incoming edges for this scenario in the fixture

        // Trace 3: Both directions
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Both,
                max_depth: 10,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.root.id.as_str(), "scen:example-build");
        assert!(!response.edges.is_empty());

        // Trace 4: Max depth of 0 should return no edges
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Outgoing,
                max_depth: 0,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.edges.len(), 0);
    }

    /// SCENARIO: Validation with different profiles
    ///
    /// GIVEN a repository with various validation issues
    /// WHEN compiling with different validation profiles
    /// THEN diagnostics should vary based on profile settings
    /// AND strict profile should escalate warnings to errors
    #[test]
    fn scenario_validation_with_different_profiles() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/orphan-scenario");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        // Compile with Default profile
        let default_graph = compile_atlas(discovered.clone(), ValidationProfile::Default);
        assert!(default_graph.metrics.error_count > 0);

        // Compile with CI profile (requires artifact producers)
        let ci_graph = compile_atlas(discovered.clone(), ValidationProfile::Ci);
        assert!(ci_graph.metrics.error_count > 0);

        // Compile with Strict profile (warnings as errors)
        let strict_graph = compile_atlas(discovered, ValidationProfile::Strict);
        assert!(strict_graph.metrics.error_count > 0);

        // Strict should have at least as many errors as Default
        assert!(strict_graph.metrics.error_count >= default_graph.metrics.error_count);
    }

    /// SCENARIO: Query returns results sorted by relevance score
    ///
    /// GIVEN a compiled atlas with multiple matching nodes
    /// WHEN performing a query
    /// THEN results should be sorted by score in descending order
    /// AND ties should be broken by ID for deterministic ordering
    #[test]
    fn scenario_query_results_sorted_by_relevance_score() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:example", NodeKind::Scenario),
                node("scen:example-test", NodeKind::Scenario),
                node("req:example", NodeKind::Requirement),
                node("cmd:example", NodeKind::Command),
            ],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "example".to_string(),
                kind: None,
            },
        );

        // Verify results are sorted by score (descending)
        let mut scores: Vec<_> = response.matches.iter().map(|m| m.score).collect();
        scores.sort_by(|a, b| b.cmp(a)); // Sort descending
        let actual_scores: Vec<_> = response.matches.iter().map(|m| m.score).collect();
        assert_eq!(
            scores, actual_scores,
            "Results should be sorted by score descending"
        );
    }

    /// SCENARIO: Trace respects max_depth parameter
    ///
    /// GIVEN a graph with multi-level relationships
    /// WHEN tracing with a limited max_depth
    /// THEN only edges up to that depth should be returned
    #[test]
    fn scenario_trace_respects_max_depth_parameter() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:root", NodeKind::Scenario),
                node("req:level1", NodeKind::Requirement),
                node("cmd:level1", NodeKind::Command),
                node("crate:level2", NodeKind::Crate),
            ],
            edges: vec![
                edge("scen:root", EdgeKind::Proves, "req:level1"),
                edge("scen:root", EdgeKind::RunsWith, "cmd:level1"),
                edge("req:level1", EdgeKind::BelongsTo, "crate:level2"),
            ],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let start_id = AtlasId::parse("scen:root").expect("valid id");

        // Trace with max_depth = 1
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Outgoing,
                max_depth: 1,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        // Should only have edges at depth 1
        assert!(response.edges.iter().all(|e| e.depth == 1));

        // Trace with max_depth = 2
        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id.clone(),
                direction: TraceDirection::Both,
                max_depth: 2,
            },
        );
        assert!(response.is_some());
        let response = response.unwrap();
        // Should have edges at depth 1 and 2
        let depths: Vec<_> = response.edges.iter().map(|e| e.depth).collect();
        assert!(depths.contains(&1));
        assert!(depths.contains(&2));
    }

    /// SCENARIO: Query with no matches returns empty results
    ///
    /// GIVEN a compiled atlas
    /// WHEN querying with a term that matches nothing
    /// THEN the response should contain no matches
    #[test]
    fn scenario_query_with_no_matches_returns_empty_results() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![
                node("scen:example", NodeKind::Scenario),
                node("req:example", NodeKind::Requirement),
            ],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let response = query_graph(
            &graph,
            &QueryRequest {
                needle: "nonexistent".to_string(),
                kind: None,
            },
        );

        assert_eq!(response.needle, "nonexistent");
        assert_eq!(response.matches.len(), 0);
    }

    /// SCENARIO: Trace with non-existent start node returns None
    ///
    /// GIVEN a compiled atlas
    /// WHEN tracing from a node ID that doesn't exist
    /// THEN the function should return None
    #[test]
    fn scenario_trace_with_nonexistent_node_returns_none() {
        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: vec![node("scen:example", NodeKind::Scenario)],
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let start_id = AtlasId::parse("node:nonexistent").expect("valid id");

        let response = trace_graph(
            &graph,
            &TraceRequest {
                start: start_id,
                direction: TraceDirection::Outgoing,
                max_depth: 10,
            },
        );

        assert!(response.is_none());
    }

    // ============================================================================
    // PROPERTY TESTS - Invariant testing with proptest
    // ============================================================================

    // PROPERTY: Deterministic ordering of graph compilation
    //
    // GIVEN the same discovered repo (same nodes, edges, config)
    // WHEN compiled multiple times
    // THEN the resulting graph should always have the same node and edge order
    proptest! {
        #[test]
        fn prop_compilation_is_deterministic(
            nodes in proptest::collection::vec(
                proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                0..10
            ),
            edges in proptest::collection::vec(
                (proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                 proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap()),
                0..10
            ),
        ) {
        use std::collections::BTreeMap;

        // Helper to create a valid node
        let create_node = |id_str: String| -> AtlasNode {
            let id = AtlasId::parse(&id_str).unwrap_or_else(|_| AtlasId::parse("req:default").unwrap());
            let kind = match id.kind_prefix() {
                "req" => NodeKind::Requirement,
                "scen" => NodeKind::Scenario,
                "cmd" => NodeKind::Command,
                "crate" => NodeKind::Crate,
                "artifact" => NodeKind::Artifact,
                "adr" => NodeKind::Adr,
                "guide" => NodeKind::Guide,
                "fixture" => NodeKind::Fixture,
                "doc" => NodeKind::Document,
                _ => NodeKind::Document,
            };
            AtlasNode {
                id,
                role: kind.role(),
                kind,
                title: id_str.clone(),
                summary: None,
                owns: vec![PathSelector::new("src/lib.rs")],
                touches: Vec::new(),
                attrs: BTreeMap::new(),
                provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
            }
        };

        let valid_nodes: Vec<_> = nodes.into_iter().map(create_node).collect();

        // Create valid edges (only between existing nodes)
        let node_ids: Vec<_> = valid_nodes.iter().map(|n| n.id.clone()).collect();
        let valid_edges: Vec<_> = edges
            .into_iter()
            .filter(|(from, to)| {
                let from_id = AtlasId::parse(from).ok();
                let to_id = AtlasId::parse(to).ok();
                from_id.is_some() && to_id.is_some() &&
                node_ids.contains(&from_id.unwrap()) &&
                node_ids.contains(&to_id.unwrap())
            })
            .map(|(from, to)| AtlasEdge {
                from: AtlasId::parse(&from).unwrap(),
                kind: EdgeKind::Explains,
                to: AtlasId::parse(&to).unwrap(),
                provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
            })
            .collect();

        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: valid_nodes.clone(),
            edges: valid_edges.clone(),
            diagnostics: vec![],
        };

        // Compile twice
        let graph1 = compile_atlas(repo.clone(), ValidationProfile::Default);
        let graph2 = compile_atlas(repo.clone(), ValidationProfile::Default);

        // Verify deterministic ordering
        assert_eq!(graph1.nodes, graph2.nodes, "Node ordering should be deterministic");
        assert_eq!(graph1.edges, graph2.edges, "Edge ordering should be deterministic");
        assert_eq!(graph1.diagnostics, graph2.diagnostics, "Diagnostic ordering should be deterministic");
    }
    }

    // PROPERTY: Edge validation for compatible node kinds
    //
    // GIVEN valid nodes and edges between compatible node kinds
    // WHEN the graph is compiled
    // THEN valid edges should always pass validation and be included in the graph
    proptest! {
        #[test]
        fn prop_valid_edges_between_compatible_kinds_pass_validation(
            kind1 in proptest::sample::select(vec![
                NodeKind::Requirement, NodeKind::Scenario, NodeKind::Command,
                NodeKind::Crate, NodeKind::Artifact, NodeKind::Adr,
            ]),
            kind2 in proptest::sample::select(vec![
                NodeKind::Requirement, NodeKind::Scenario, NodeKind::Command,
                NodeKind::Crate, NodeKind::Artifact, NodeKind::Adr,
            ]),
        ) {
        let node1 = node("node:one", kind1);
        let node2 = node("node:two", kind2);

        // Try all edge kinds
        for edge_kind in [
            EdgeKind::Explains, EdgeKind::Proves, EdgeKind::UsesFixture,
            EdgeKind::RunsWith, EdgeKind::Emits, EdgeKind::Exercises,
            EdgeKind::Documents, EdgeKind::BelongsTo,
        ] {
            let test_edge = edge("node:one", edge_kind, "node:two");

            let repo = DiscoveredRepo {
                repo: RepoDescriptor {
                    name: "sample".to_string(),
                },
                config: AtlasConfig::default(),
                nodes: vec![node1.clone(), node2.clone()],
                edges: vec![test_edge],
                diagnostics: vec![],
            };

            let graph = compile_atlas(repo, ValidationProfile::Default);

            // If the edge is valid according to edge_endpoint_is_valid, it should be in the graph
            let is_valid = edge_endpoint_is_valid(edge_kind, kind1, kind2);
            let edge_in_graph = graph.edges.iter().any(|e| e.kind == edge_kind && e.from == node1.id && e.to == node2.id);

            // For valid edge configurations, the edge should be present
            if is_valid {
                assert!(edge_in_graph, "Valid edge {:?} between {:?} and {:?} should be in graph", edge_kind, kind1, kind2);
            }
        }
        }
    }

    // PROPERTY: ID uniqueness detection is order-independent
    //
    // GIVEN nodes with duplicate IDs in any order
    // WHEN the graph is compiled
    // THEN duplicate IDs should always be detected regardless of order
    proptest! {
        #[test]
        fn prop_duplicate_id_detection_is_order_independent(
            duplicate_id in proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
            other_ids in proptest::collection::vec(
                proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                0..5
            ),
        ) {
        use std::collections::BTreeMap;

        let dup_id = AtlasId::parse(&duplicate_id).unwrap_or_else(|_| AtlasId::parse("req:dup").unwrap());

        // Create two nodes with the same ID
        let node1 = AtlasNode {
            id: dup_id.clone(),
            kind: NodeKind::Requirement,
            role: NodeKind::Requirement.role(),
            title: "First".to_string(),
            summary: None,
            owns: vec![PathSelector::new("src/first.rs")],
            touches: Vec::new(),
            attrs: BTreeMap::new(),
            provenance: Provenance::new(RepoRelativePath::new("atlas/first.atlas.yaml")),
        };

        let node2 = AtlasNode {
            id: dup_id.clone(),
            kind: NodeKind::Requirement,
            role: NodeKind::Requirement.role(),
            title: "Second".to_string(),
            summary: None,
            owns: vec![PathSelector::new("src/second.rs")],
            touches: Vec::new(),
            attrs: BTreeMap::new(),
            provenance: Provenance::new(RepoRelativePath::new("atlas/second.atlas.yaml")),
        };

        let other_nodes: Vec<_> = other_ids
            .into_iter()
            .filter_map(|id_str| {
                let id = AtlasId::parse(&id_str).ok()?;
                if id == dup_id {
                    None
                } else {
                    Some(node(id.as_str(), NodeKind::Requirement))
                }
            })
            .collect();

        // Test with duplicate in different positions
        for i in 0..=other_nodes.len() {
            let mut nodes = other_nodes.clone();
            nodes.insert(i, node1.clone());
            nodes.insert(i + 1, node2.clone());

            let repo = DiscoveredRepo {
                repo: RepoDescriptor {
                    name: "sample".to_string(),
                },
                config: AtlasConfig::default(),
                nodes,
                edges: vec![],
                diagnostics: vec![],
            };

            let graph = compile_atlas(repo, ValidationProfile::Default);

            // Duplicate ID should always be detected
            assert!(
                graph.diagnostics.iter().any(|d| d.code == DiagnosticCode::DuplicateId),
                "Duplicate ID should be detected regardless of position"
            );

            // Only one node with duplicate ID should exist
            let count = graph.nodes.iter().filter(|n| n.id == dup_id).count();
            assert_eq!(count, 1, "Only one instance of duplicate ID should exist");
        }
        }
    }

    // PROPERTY: Query determinism
    //
    // GIVEN a compiled graph and a query request
    // WHEN the same query is executed multiple times
    // THEN the results should always be identical
    proptest! {
        #[test]
        fn prop_query_is_deterministic(
            nodes in proptest::collection::vec(
                proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                1..10
            ),
            needle in "[a-z0-9_-]{1,20}",
        ) {
        use std::collections::BTreeMap;

        let valid_nodes: Vec<_> = nodes
            .into_iter()
            .filter_map(|id_str| {
                let id = AtlasId::parse(&id_str).ok()?;
                Some(AtlasNode {
                    id: id.clone(),
                    kind: NodeKind::Requirement,
                    role: NodeKind::Requirement.role(),
                    title: format!("Title for {}", id_str),
                    summary: Some(format!("Summary for {}", id_str)),
                    owns: vec![PathSelector::new("src/lib.rs")],
                    touches: Vec::new(),
                    attrs: BTreeMap::new(),
                    provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
                })
            })
            .collect();

        proptest::prelude::prop_assume!(!valid_nodes.is_empty());

        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: valid_nodes,
            edges: vec![],
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);
        let request = QueryRequest {
            needle: needle.clone(),
            kind: None,
        };

        // Execute query multiple times
        let response1 = query_graph(&graph, &request);
        let response2 = query_graph(&graph, &request);
        let response3 = query_graph(&graph, &request);

        // All responses should be identical
        assert_eq!(response1, response2, "Query should be deterministic");
        assert_eq!(response2, response3, "Query should be deterministic");
        }
    }

    // PROPERTY: Trace determinism
    //
    // GIVEN a compiled graph and a trace request
    // WHEN the same trace is executed multiple times
    // THEN the results should always be identical
    proptest! {
        #[test]
        fn prop_trace_is_deterministic(
            nodes in proptest::collection::vec(
                proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                2..10
            ),
            max_depth in 0usize..5,
            direction_idx in 0usize..3,
        ) {
        use std::collections::BTreeMap;

        let valid_nodes: Vec<_> = nodes
            .into_iter()
            .filter_map(|id_str: String| {
                let id = AtlasId::parse(&id_str).ok()?;
                Some(AtlasNode {
                    id: id.clone(),
                    kind: NodeKind::Requirement,
                    role: NodeKind::Requirement.role(),
                    title: id_str.clone(),
                    summary: None,
                    owns: vec![PathSelector::new("src/lib.rs")],
                    touches: Vec::new(),
                    attrs: BTreeMap::new(),
                    provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
                })
            })
            .collect();

        proptest::prelude::prop_assume!(valid_nodes.len() >= 2);

        // Create edges between consecutive nodes
        let mut edges = Vec::new();
        for i in 0..valid_nodes.len() - 1 {
            edges.push(AtlasEdge {
                from: valid_nodes[i].id.clone(),
                kind: EdgeKind::Explains,
                to: valid_nodes[i + 1].id.clone(),
                provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
            });
        }

        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: valid_nodes,
            edges,
            diagnostics: vec![],
        };

        let graph = compile_atlas(repo, ValidationProfile::Default);

        let direction = match direction_idx {
            0 => TraceDirection::Outgoing,
            1 => TraceDirection::Incoming,
            _ => TraceDirection::Both,
        };

        let request = TraceRequest {
            start: graph.nodes[0].id.clone(),
            direction,
            max_depth,
        };

        // Execute trace multiple times
        let response1 = trace_graph(&graph, &request);
        let response2 = trace_graph(&graph, &request);
        let response3 = trace_graph(&graph, &request);

        // All responses should be identical
        assert_eq!(response1, response2, "Trace should be deterministic");
        assert_eq!(response2, response3, "Trace should be deterministic");
        }
    }

    // PROPERTY: Profile escalation
    //
    // GIVEN the same discovered repo
    // WHEN compiled with different validation profiles
    // THEN stricter profiles should catch more or equal diagnostics than looser profiles
    proptest! {
        #[test]
        fn prop_stricter_profile_catches_more_diagnostics(
            nodes in proptest::collection::vec(
                proptest::string::string_regex("[a-z]+:[a-z0-9_-]+").unwrap(),
                1..10
            ),
        ) {
        use std::collections::BTreeMap;

        let valid_nodes: Vec<_> = nodes
            .into_iter()
            .filter_map(|id_str: String| {
                let id = AtlasId::parse(&id_str).ok()?;
                let kind = match id.kind_prefix() {
                    "scen" => NodeKind::Scenario,
                    "cmd" => NodeKind::Command,
                    "crate" => NodeKind::Crate,
                    "artifact" => NodeKind::Artifact,
                    _ => NodeKind::Requirement,
                };
                Some(AtlasNode {
                    id: id.clone(),
                    kind,
                    role: kind.role(),
                    title: id_str.clone(),
                    summary: None,
                    owns: vec![PathSelector::new("src/lib.rs")],
                    touches: Vec::new(),
                    attrs: BTreeMap::new(),
                    provenance: Provenance::new(RepoRelativePath::new("atlas/example.atlas.yaml")),
                })
            })
            .collect();

        proptest::prelude::prop_assume!(!valid_nodes.is_empty());

        let repo = DiscoveredRepo {
            repo: RepoDescriptor {
                name: "sample".to_string(),
            },
            config: AtlasConfig::default(),
            nodes: valid_nodes,
            edges: vec![],
            diagnostics: vec![],
        };

        // Compile with different profiles
        let default_graph = compile_atlas(repo.clone(), ValidationProfile::Default);
        let ci_graph = compile_atlas(repo.clone(), ValidationProfile::Ci);
        let strict_graph = compile_atlas(repo, ValidationProfile::Strict);

        // Stricter profiles should catch more or equal diagnostics
        // CI profile requires artifact producers, so it should catch at least as much as Default
        assert!(
            ci_graph.metrics.error_count >= default_graph.metrics.error_count,
            "CI profile should catch at least as many errors as Default"
        );

        // Strict profile escalates warnings to errors, so it should catch at least as much as CI
        assert!(
            strict_graph.metrics.error_count >= ci_graph.metrics.error_count,
            "Strict profile should catch at least as many errors as CI"
        );

        // Strict should catch at least as much as Default
        assert!(
            strict_graph.metrics.error_count >= default_graph.metrics.error_count,
            "Strict profile should catch at least as many errors as Default"
        );
        }
    }
}

#[cfg(test)]
mod golden {
    use super::*;
    use atlasctl_discover_fs::FsDiscovery;
    use atlasctl_fixtures::repo;
    use atlasctl_ports::{DiscoverRequest, DiscoveryPort, RenderPort};
    use atlasctl_render::AtlasRenderer;
    use atlasctl_types::{ChangedPath, RenderFormat, WhyRequest, WhySubject};
    use serde_json::Value;

    const FIXTURES: &[&str] = &[
        "valid-minimal",
        "broken-link",
        "duplicate-id",
        "orphan-scenario",
        "markdown-frontmatter",
        "doctor-drift",
        "requirement-unproven",
        "overlapping-participation",
    ];

    fn build_atlas(name: &str) -> AtlasGraph {
        let repo_path = repo(name);
        let discovery = FsDiscovery;
        let request = DiscoverRequest {
            repo_root: repo_path,
            config_path: None,
        };

        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        compile_atlas(discovered, ValidationProfile::Default)
    }

    #[test]
    fn scenario_why_finds_proof_chain() {
        let graph = build_atlas("valid-minimal");
        let id = AtlasId::parse("scen:example-build").unwrap();
        let request = WhyRequest {
            subject: WhySubject::Id(id),
        };
        let response = why_graph(&graph, &request).expect("response");
        assert_eq!(response.root.id.as_str(), "scen:example-build");
        assert!(!response.chain.is_empty());
        assert!(
            response
                .chain
                .iter()
                .any(|s| s.node.id.as_str() == "crate:engine")
        );
    }

    #[test]
    fn scenario_why_by_path() {
        let graph = build_atlas("valid-minimal");
        let request = WhyRequest {
            subject: WhySubject::Path("crates/engine".into()),
        };
        let response = why_graph(&graph, &request).expect("response");
        assert_eq!(response.root.id.as_str(), "crate:engine");
    }

    #[test]
    fn portability_no_absolute_paths_in_output() {
        let graph = build_atlas("valid-minimal");
        let json = serde_json::to_string(&graph).unwrap();

        // Assert no absolute paths (roughly starting with / or [A-Z]:\)
        // This is a simple heuristic but effective for regression
        assert!(!json.contains(":\\\\"), "Found Windows absolute path");
        assert!(!json.contains("\":/"), "Found POSIX absolute path");

        let renderer = AtlasRenderer;
        let why = {
            let request = WhyRequest {
                subject: WhySubject::Id(AtlasId::parse("scen:example-build").unwrap()),
            };
            render_why_json(&renderer, &graph, &request)
        };
        assert!(
            !why.contains(":\\\\"),
            "Found Windows absolute path in why.json"
        );
        assert!(
            !why.contains("\":/"),
            "Found POSIX absolute path in why.json"
        );

        let impact = {
            let request = ImpactRequest {
                paths: vec![ChangedPath {
                    path: "crates/engine/src/lib.rs".into(),
                }],
                owners: BTreeMap::new(),
            };
            render_impact_json(&renderer, &graph, &request)
        };
        assert!(
            !impact.contains(":\\\\"),
            "Found Windows absolute path in impact.json"
        );
        assert!(
            !impact.contains("\":/"),
            "Found POSIX absolute path in impact.json"
        );
    }

    #[test]
    fn portability_slashes_are_normalized() {
        let graph = build_atlas("valid-minimal");
        for node in &graph.nodes {
            for path in node.all_paths() {
                assert!(
                    !path.pattern.contains('\\'),
                    "Path selector contains backslash: {}",
                    path.pattern
                );
            }
            assert!(
                !node.provenance.source.as_str().contains('\\'),
                "Provenance contains backslash: {}",
                node.provenance.source
            );
        }
    }

    #[test]
    fn scenario_impacted_by_path() {
        let graph = build_atlas("valid-minimal");
        let request = ImpactRequest {
            paths: vec![ChangedPath {
                path: "crates/engine".into(),
            }],
            owners: BTreeMap::new(),
        };
        let response = impacted_graph(&graph, &request);
        assert!(!response.impacted.is_empty());
        assert!(
            response
                .impacted
                .iter()
                .any(|h| h.node.id.as_str() == "crate:engine")
        );
        // Scenario exercises engine, so it should be impacted too
        assert!(
            response
                .impacted
                .iter()
                .any(|h| h.node.id.as_str() == "scen:example-build")
        );
    }

    #[test]
    fn scenario_impacted_uncovered() {
        let graph = build_atlas("valid-minimal");
        let request = ImpactRequest {
            paths: vec![ChangedPath {
                path: "unknown/file.txt".into(),
            }],
            owners: BTreeMap::new(),
        };
        let response = impacted_graph(&graph, &request);
        assert!(response.impacted.is_empty());
        assert!(!response.uncovered.is_empty());
        assert_eq!(response.uncovered[0].path.as_str(), "unknown/file.txt");
    }

    #[test]
    fn scenario_validation_with_new_classes() {
        use atlasctl_discover_fs::FsDiscovery;
        use atlasctl_ports::DiscoverRequest;
        use camino::Utf8PathBuf;

        let repo_root = Utf8PathBuf::from("../../fixtures/repos/requirement-unproven");
        let request = DiscoverRequest {
            repo_root,
            config_path: None,
        };

        let discovery = FsDiscovery;
        let discovered = discovery
            .discover(&request)
            .expect("discovery should succeed");

        // Compile with CI profile which has the new checks enabled
        let graph = compile_atlas(discovered, ValidationProfile::Ci);

        assert!(
            graph
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::RequirementNotProven)
        );
        assert!(
            graph
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::UncoveredCrate)
        );
        assert!(graph.metrics.error_count >= 2);
    }

    #[test]
    fn golden_json_output() {
        let renderer = AtlasRenderer;

        for fixture in FIXTURES {
            let graph = build_atlas(fixture);
            let json_output = renderer
                .render(&graph, RenderFormat::Json)
                .expect("rendering JSON should succeed");

            insta::assert_snapshot!(format!("golden/{}.json", fixture), json_output);
        }
    }

    #[test]
    fn golden_markdown_output() {
        let renderer = AtlasRenderer;

        for fixture in FIXTURES {
            let graph = build_atlas(fixture);
            let md_output = renderer
                .render(&graph, RenderFormat::Markdown)
                .expect("rendering Markdown should succeed");

            insta::assert_snapshot!(format!("golden/{}.md", fixture), md_output);
        }
    }

    #[test]
    fn golden_why_output() {
        let renderer = AtlasRenderer;
        let graph = build_atlas("valid-minimal");
        let id = AtlasId::parse("scen:example-build").unwrap();
        let request = WhyRequest {
            subject: WhySubject::Id(id),
        };
        let response = why_graph(&graph, &request).expect("response");

        let md = renderer
            .render_why(&response, RenderFormat::Markdown)
            .unwrap();
        insta::assert_snapshot!("golden/why.md", md);

        let json = renderer.render_why(&response, RenderFormat::Json).unwrap();
        let json_value: Value =
            serde_json::from_str(&json).expect("why output should be valid JSON envelope");
        assert_eq!(
            json_value.get("schema_version").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            json_value.get("command").and_then(Value::as_str),
            Some("why")
        );
        assert!(json_value.get("payload").is_some());
        insta::assert_snapshot!("golden/why.json", json);
    }

    #[test]
    fn golden_impact_output() {
        let renderer = AtlasRenderer;
        let graph = build_atlas("valid-minimal");
        let request = ImpactRequest {
            paths: vec![ChangedPath {
                path: "crates/engine/src/lib.rs".into(),
            }],
            owners: BTreeMap::new(),
        };
        let response = impacted_graph(&graph, &request);

        let md = renderer
            .render_impact(&response, RenderFormat::Markdown)
            .unwrap();
        insta::assert_snapshot!("golden/impact.md", md);

        let json = renderer
            .render_impact(&response, RenderFormat::Json)
            .unwrap();
        let json_value: Value =
            serde_json::from_str(&json).expect("impact output should be valid JSON envelope");
        assert_eq!(
            json_value.get("schema_version").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            json_value.get("command").and_then(Value::as_str),
            Some("impacted")
        );
        assert!(json_value.get("payload").is_some());
        insta::assert_snapshot!("golden/impact.json", json);

        let summary = renderer
            .render_impact(&response, RenderFormat::GitHubSummary)
            .unwrap();
        insta::assert_snapshot!("golden/impact-summary.md", summary);

        let packet = renderer
            .render_impact(&response, RenderFormat::ReviewPacket)
            .unwrap();
        insta::assert_snapshot!("golden/impact-packet.md", packet);
    }

    fn render_why_json(
        renderer: &AtlasRenderer,
        graph: &AtlasGraph,
        request: &WhyRequest,
    ) -> String {
        let response = why_graph(graph, request).expect("why response");
        renderer.render_why(&response, RenderFormat::Json).unwrap()
    }

    fn render_impact_json(
        renderer: &AtlasRenderer,
        graph: &AtlasGraph,
        request: &ImpactRequest,
    ) -> String {
        let response = impacted_graph(graph, request);
        renderer
            .render_impact(&response, RenderFormat::Json)
            .unwrap()
    }
}
