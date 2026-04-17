#![forbid(unsafe_code)]

use atlasctl_ports::{RenderError, RenderPort};
use atlasctl_types::{
    AtlasDiagnostic, AtlasGraph, AtlasNode, ImpactResponse, NodeKind, RenderFormat, WhyResponse,
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
            RenderFormat::Json => serde_json::to_string_pretty(response).map_err(|err| {
                RenderError::Message(format!("failed to render why response as JSON: {err}"))
            }),
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
            RenderFormat::Json => serde_json::to_string_pretty(response).map_err(|err| {
                RenderError::Message(format!("failed to render impact response as JSON: {err}"))
            }),
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

    out.push_str("## Nodes by kind\n\n");
    for kind in [
        NodeKind::Requirement,
        NodeKind::Adr,
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
        "This packet summarizes the behavioral and proof surface impacted by these changes.\n\n",
    );

    out.push_str("## 🎯 Impacted Proof Surface\n\n");
    if response.impacted.is_empty() {
        out.push_str("_No proof surface impacted._\n");
    } else {
        for hit in &response.impacted {
            let role_emoji = match hit.node.role {
                atlasctl_types::NodeRole::Behavior => "🎯",
                atlasctl_types::NodeRole::Proof => "🧱",
                atlasctl_types::NodeRole::Document => "📄",
                atlasctl_types::NodeRole::Artifact => "📦",
                atlasctl_types::NodeRole::Command => "🤖",
                atlasctl_types::NodeRole::Infra => "🏗️",
            };

            out.push_str(&format!(
                "### {} `{}` ({})\n",
                role_emoji, hit.node.id, hit.node.kind
            ));
            out.push_str(&format!("- **Title**: {}\n", hit.node.title));
            out.push_str(&format!("- **Reason**: {}\n", hit.reason));

            if !hit.owners.is_empty() {
                out.push_str(&format!("- **Reviewers**: {}\n", hit.owners.join(", ")));
            }

            if let Some(summary) = &hit.node.summary {
                out.push_str("\n#### Summary\n\n");
                out.push_str(summary);
                out.push('\n');
            }

            if !hit.node.owns.is_empty() {
                out.push_str("\n#### Owns\n\n");
                for p in &hit.node.owns {
                    out.push_str(&format!("- `{}`\n", p.pattern));
                }
            }

            out.push('\n');
        }
    }

    if !response.uncovered.is_empty() {
        out.push_str("## 🔍 Uncovered Changes\n\n");
        out.push_str("The following changed paths are not covered by any node in the atlas:\n\n");
        for path in &response.uncovered {
            out.push_str(&format!("- `{}`\n", path.path));
        }
        out.push('\n');
    }

    out.push_str("---\n_Generated by atlasctl_\n");

    out
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
