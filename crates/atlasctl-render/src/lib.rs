#![forbid(unsafe_code)]

use atlasctl_app::{RenderError, RenderPort};
use atlasctl_types::{
    AtlasDiagnostic, AtlasGraph, AtlasNode, EdgeKind, ImpactEnvelope, ImpactResponse, NodeKind,
    RenderFormat, WhyEnvelope, WhyResponse,
};

#[derive(Debug, Default)]
pub struct AtlasRenderer;

impl RenderPort for AtlasRenderer {
    fn render(&self, graph: &AtlasGraph, format: RenderFormat) -> Result<String, RenderError> {
        match format {
            RenderFormat::Json => serde_json::to_string_pretty(graph)
                .map_err(|err| RenderError::Message(format!("failed to render JSON: {err}"))),
            RenderFormat::Markdown => Ok(render_markdown(graph)),
            RenderFormat::GitHubSummary | RenderFormat::ReviewPacket => {
                Ok(render_gh_summary(graph))
            }
        }
    }

    fn render_why(
        &self,
        response: &WhyResponse,
        format: RenderFormat,
    ) -> Result<String, RenderError> {
        match format {
            RenderFormat::Json => {
                let envelope = WhyEnvelope::for_command("why", response.clone());
                serde_json::to_string_pretty(&envelope).map_err(|err| {
                    RenderError::Message(format!("failed to render why response as JSON: {err}"))
                })
            }
            RenderFormat::Markdown | RenderFormat::GitHubSummary | RenderFormat::ReviewPacket => {
                Ok(render_why_markdown(response))
            }
        }
    }

    fn render_impact(
        &self,
        response: &ImpactResponse,
        format: RenderFormat,
    ) -> Result<String, RenderError> {
        match format {
            RenderFormat::Json => {
                let envelope = ImpactEnvelope::for_command("impacted", response.clone());
                serde_json::to_string_pretty(&envelope).map_err(|err| {
                    RenderError::Message(format!("failed to render impact response as JSON: {err}"))
                })
            }
            RenderFormat::Markdown => Ok(render_impact_markdown(response)),
            RenderFormat::GitHubSummary => Ok(render_impact_gh_summary(response)),
            RenderFormat::ReviewPacket => Ok(render_review_packet(response)),
        }
    }
}

fn render_markdown(graph: &AtlasGraph) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Atlas: {}\n\n", graph.repo.name));
    out.push_str("## Summary\n\n");
    out.push_str(&format!("- Schema version: `{}`\n", graph.schema_version));
    out.push_str(&format!("- Tool version: `{}`\n", graph.tool_version));
    out.push_str(&format!("- Nodes: `{}`\n", graph.metrics.node_count));
    out.push_str(&format!("- Edges: `{}`\n", graph.metrics.edge_count));
    out.push_str(&format!(
        "- Diagnostics: `{}` (errors: `{}`, warnings: `{}`)\n\n",
        graph.metrics.diagnostic_count, graph.metrics.error_count, graph.metrics.warning_count
    ));

    out.push_str("## Scenario Index\n\n");
    let scenarios: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Scenario)
        .collect();

    if scenarios.is_empty() {
        out.push_str("_No scenarios defined._\n\n");
    } else {
        out.push_str("| Scenario | Proves | Exercises |\n");
        out.push_str("| --- | --- | --- |\n");
        for scen in scenarios {
            let proves: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.from == scen.id && e.kind == EdgeKind::Proves)
                .map(|e| format!("`{}`", e.to))
                .collect();
            let exercises: Vec<_> = graph
                .edges
                .iter()
                .filter(|e| e.from == scen.id && e.kind == EdgeKind::Exercises)
                .map(|e| format!("`{}`", e.to))
                .collect();

            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                scen.id,
                proves.join(", "),
                exercises.join(", ")
            ));
        }
        out.push('\n');
    }

    out.push_str("## Nodes by kind\n\n");
    for kind in [
        NodeKind::Requirement,
        NodeKind::Roadmap,
        NodeKind::Proposal,
        NodeKind::Spec,
        NodeKind::Adr,
        NodeKind::Plan,
        NodeKind::Goal,
        NodeKind::SupportTier,
        NodeKind::PolicyLedger,
        NodeKind::Closeout,
        NodeKind::Claim,
        NodeKind::Guide,
        NodeKind::Scenario,
        NodeKind::Fixture,
        NodeKind::Command,
        NodeKind::Artifact,
        NodeKind::Crate,
        NodeKind::Document,
    ] {
        let nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|node| node.kind == kind)
            .collect();
        if nodes.is_empty() {
            continue;
        }

        out.push_str(&format!("### {}\n\n", kind));
        for node in nodes {
            render_node(node, &mut out);
        }
    }

    out.push_str("## Edges\n\n");
    for edge in &graph.edges {
        out.push_str(&format!(
            "- `{}` --{}--> `{}`\n",
            edge.from, edge.kind, edge.to
        ));
    }
    out.push('\n');

    out.push_str("## Diagnostics\n\n");
    if graph.diagnostics.is_empty() {
        out.push_str("_No diagnostics._\n");
    } else {
        for diagnostic in &graph.diagnostics {
            render_diagnostic(diagnostic, &mut out);
        }
    }

    out
}

fn render_impact_markdown(response: &ImpactResponse) -> String {
    let mut out = String::new();

    out.push_str("# Impact Analysis\n\n");

    out.push_str("## Impacted Nodes\n\n");
    if response.impacted.is_empty() {
        out.push_str("_No nodes impacted._\n");
    } else {
        for hit in &response.impacted {
            out.push_str(&format!(
                "- `{}` ({}) — {}\n",
                hit.node.id, hit.node.kind, hit.node.title
            ));
            out.push_str(&format!("  - Reason: {}\n", hit.reason));
            if !hit.owners.is_empty() {
                out.push_str(&format!("  - Owners: {}\n", hit.owners.join(", ")));
            }
        }
    }

    out.push_str("\n## Uncovered Changes\n\n");
    if response.uncovered.is_empty() {
        out.push_str("_All changes are covered by the atlas._\n");
    } else {
        for path in &response.uncovered {
            out.push_str(&format!("- `{}`\n", path.path));
        }
    }

    out
}

fn render_review_packet(response: &ImpactResponse) -> String {
    let mut out = String::new();

    out.push_str("# 📦 Atlas Review Packet\n\n");
    out.push_str(
        "This packet summarizes what changed, what surface is impacted, and what proof evidence is missing.\n\n",
    );

    out.push_str("## 📂 Changed Paths\n\n");
    if response.changed_paths.is_empty() {
        out.push_str("_No paths provided._\n\n");
    } else {
        for path in &response.changed_paths {
            out.push_str(&format!("- `{}`\n", path.path));
        }
        out.push('\n');
    }

    out.push_str("## 🧭 Impacted Truth Surface\n\n");
    render_impact_kind_group(
        &mut out,
        "Roadmaps / Proposals / Specs / ADRs / Plans / Goals",
        response,
        |kind| {
            matches!(
                kind,
                NodeKind::Roadmap
                    | NodeKind::Proposal
                    | NodeKind::Spec
                    | NodeKind::Adr
                    | NodeKind::Plan
                    | NodeKind::Goal
            )
        },
    );
    render_impact_kind_group(
        &mut out,
        "Requirements / Scenarios / Closeouts",
        response,
        |kind| {
            matches!(
                kind,
                NodeKind::Requirement | NodeKind::Scenario | NodeKind::Closeout
            )
        },
    );
    render_impact_kind_group(&mut out, "Policy / Support / Claims", response, |kind| {
        matches!(
            kind,
            NodeKind::PolicyLedger | NodeKind::SupportTier | NodeKind::Claim
        )
    });
    render_impact_kind_group(&mut out, "Proof Commands", response, |kind| {
        kind == NodeKind::Command
    });
    render_impact_kind_group(&mut out, "Artifacts / Infra", response, |kind| {
        matches!(kind, NodeKind::Artifact | NodeKind::Crate)
    });

    out.push_str("## 🧪 Proof Commands to Run\n\n");
    let commands: Vec<_> = response
        .impacted
        .iter()
        .filter(|hit| hit.node.kind == NodeKind::Command)
        .collect();
    if commands.is_empty() {
        out.push_str("_No command nodes are currently impacted._\n\n");
    } else {
        for hit in commands {
            out.push_str(&format!("- `{}` — {}\n", hit.node.id, hit.node.title));
        }
        out.push('\n');
    }

    if !response.uncovered.is_empty() {
        out.push_str("## 🔍 Uncovered Changes\n\n");
        out.push_str("These paths are not covered by any node in the atlas:\n\n");
        for path in &response.uncovered {
            out.push_str(&format!("- `{}`\n", path.path));
        }
        out.push('\n');
    }

    out.push_str("## ⚠️ Missing Evidence\n\n");
    if response.missing_evidence.is_empty() {
        out.push_str("_No immediate missing-evidence diagnostics for impacted nodes._\n\n");
    } else {
        for diagnostic in &response.missing_evidence {
            out.push_str(&format!(
                "- `{}.{}`: {}\n",
                diagnostic.severity, diagnostic.code, diagnostic.message
            ));
            if let Some(subject) = &diagnostic.subject {
                out.push_str(&format!("  - subject: `{}`\n", subject));
            }
            if let Some(location) = &diagnostic.location {
                out.push_str(&format!("  - location: `{}`\n", location.path));
            }
        }
        out.push('\n');
    }

    out.push_str("## ⚠️ Scope Warnings\n\n");
    if response.scope_warnings.is_empty() {
        out.push_str("_No extra scope warnings detected for this impact._\n\n");
    } else {
        for warning in &response.scope_warnings {
            out.push_str(&format!("- {}\n", warning));
        }
        out.push('\n');
    }

    out.push_str("## ✅ Next Actions\n\n");
    if response.suggested_fixes.is_empty() {
        out.push_str("_No follow-up metadata fixes are inferred from current diagnostics._\n\n");
    } else {
        for fix in &response.suggested_fixes {
            out.push_str(&format!("- {fix}\n"));
        }
        out.push('\n');
    }

    out.push_str("---\n_Generated by atlasctl_\n");

    out
}

fn render_impact_kind_group(
    out: &mut String,
    heading: &str,
    response: &ImpactResponse,
    predicate: impl Fn(NodeKind) -> bool,
) {
    out.push_str(&format!("### {}\n\n", heading));
    let hits: Vec<_> = response
        .impacted
        .iter()
        .filter(|hit| predicate(hit.node.kind))
        .collect();
    if hits.is_empty() {
        out.push_str("_None_\n\n");
        return;
    }
    for hit in hits {
        out.push_str(&format!(
            "- `{}` ({}) — {}\n",
            hit.node.id, hit.node.kind, hit.node.title
        ));
        let reason = hit.reason.replace('`', "\\`");
        out.push_str(&format!("  - reason: `{}`\n", reason));
        if !hit.owners.is_empty() {
            out.push_str(&format!("  - owners: `{}`\n", hit.owners.join(", ")));
        }
    }
    out.push('\n');
}

fn render_gh_summary(graph: &AtlasGraph) -> String {
    let mut out = String::new();

    out.push_str("### 🗺️ Atlas Summary\n\n");
    out.push_str(&format!("- **Repository**: `{}`\n", graph.repo.name));
    out.push_str(&format!(
        "- **Inventory**: `{}` nodes, `{}` edges\n",
        graph.metrics.node_count, graph.metrics.edge_count
    ));

    if graph.metrics.diagnostic_count > 0 {
        let status = if graph.metrics.error_count > 0 {
            "🔴 Failed"
        } else {
            "⚠️  Warning"
        };
        out.push_str(&format!(
            "- **Status**: {} (`{}` errors, `{}` warnings)\n",
            status, graph.metrics.error_count, graph.metrics.warning_count
        ));

        out.push_str("\n#### ⚠️ Top Diagnostics\n\n");
        for diagnostic in graph.diagnostics.iter().take(5) {
            out.push_str(&format!(
                "- `{}` — {}\n",
                diagnostic.code, diagnostic.message
            ));
        }
    } else {
        out.push_str("- **Status**: ✅ Healthy\n");
    }

    out
}

fn render_impact_gh_summary(response: &ImpactResponse) -> String {
    let mut out = String::new();

    out.push_str("### 🎯 Atlas Impact Analysis\n\n");

    out.push_str(&format!(
        "- **Impacted Behaviors**: `{}`\n",
        response.impacted.len()
    ));
    out.push_str(&format!(
        "- **Uncovered Changes**: `{}`\n",
        response.uncovered.len()
    ));

    if !response.impacted.is_empty() {
        out.push_str("\n#### 🧱 Impacted Proof Surface\n\n");
        for hit in response.impacted.iter().take(10) {
            let owners = if hit.owners.is_empty() {
                "".to_string()
            } else {
                format!(" (👥 {})", hit.owners.join(", "))
            };
            out.push_str(&format!(
                "- `{}` ({}) — {}{}\n",
                hit.node.id, hit.node.kind, hit.node.title, owners
            ));
        }
        if response.impacted.len() > 10 {
            out.push_str(&format!(
                "\n_... and {} more impacted nodes._\n",
                response.impacted.len() - 10
            ));
        }
    }

    if !response.uncovered.is_empty() {
        out.push_str("\n#### 🔍 Uncovered changed paths\n\n");
        for path in response.uncovered.iter().take(5) {
            out.push_str(&format!("- `{}`\n", path.path));
        }
        if response.uncovered.len() > 5 {
            out.push_str(&format!(
                "\n_... and {} more uncovered paths._\n",
                response.uncovered.len() - 5
            ));
        }
    }

    out
}

fn render_why_markdown(response: &WhyResponse) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Why: `{}`\n\n", response.root.id));
    out.push_str(&format!("- **Title**: {}\n", response.root.title));
    out.push_str(&format!("- **Kind**: `{}`\n", response.root.kind));
    out.push_str(&format!(
        "- **Source**: `{}`\n",
        response.root.provenance.source
    ));

    if let Some(summary) = &response.root.summary {
        out.push_str(&format!("\n## Summary\n\n{}\n", summary));
    }

    if !response.root.owns.is_empty() {
        out.push_str("\n## Owns\n\n");
        for path in &response.root.owns {
            out.push_str(&format!("- `{}`\n", path.pattern));
        }
    }

    if !response.root.touches.is_empty() {
        out.push_str("\n## Touches\n\n");
        for path in &response.root.touches {
            out.push_str(&format!("- `{}`\n", path.pattern));
        }
    }

    out.push_str("\n## Proof Chain\n\n");
    if response.chain.is_empty() {
        out.push_str("_No immediate proof chain found._\n");
    } else {
        for step in &response.chain {
            let direction = match step.direction {
                atlasctl_types::TraceDirection::Incoming => "is supported by",
                atlasctl_types::TraceDirection::Outgoing => "is exercised by",
                atlasctl_types::TraceDirection::Both => "relates to",
            };
            out.push_str(&format!(
                "- `{}` {} `{}` (via `{}`)\n",
                response.root.id, direction, step.node.id, step.relationship
            ));
        }
    }

    out
}

fn render_node(node: &AtlasNode, out: &mut String) {
    out.push_str(&format!("- `{}` — {}\n", node.id, node.title));
    out.push_str(&format!("  - Source: `{}`\n", node.provenance.source));
    if let Some(summary) = &node.summary {
        out.push_str(&format!("  - Summary: {}\n", summary));
    }
    if !node.owns.is_empty() {
        let joined = node
            .owns
            .iter()
            .map(|path| format!("`{}`", path.pattern))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  - Owns: {}\n", joined));
    }
    if !node.touches.is_empty() {
        let joined = node
            .touches
            .iter()
            .map(|path| format!("`{}`", path.pattern))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  - Touches: {}\n", joined));
    }
    out.push('\n');
}

fn render_diagnostic(diagnostic: &AtlasDiagnostic, out: &mut String) {
    out.push_str(&format!(
        "- [{}] `{}` — {}\n",
        diagnostic.severity, diagnostic.code, diagnostic.message
    ));

    if let Some(subject) = &diagnostic.subject {
        out.push_str(&format!("  - Subject: `{}`\n", subject));
    }

    if let Some(location) = &diagnostic.location {
        out.push_str(&format!("  - Location: `{}`\n", location.path));
    }
}
