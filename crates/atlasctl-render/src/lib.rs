#![forbid(unsafe_code)]

use atlasctl_ports::{RenderError, RenderPort};
use atlasctl_types::{AtlasDiagnostic, AtlasGraph, AtlasNode, NodeKind, RenderFormat};

#[derive(Debug, Default)]
pub struct AtlasRenderer;

impl RenderPort for AtlasRenderer {
    fn render(&self, graph: &AtlasGraph, format: RenderFormat) -> Result<String, RenderError> {
        match format {
            RenderFormat::Json => serde_json::to_string_pretty(graph)
                .map_err(|err| RenderError::Message(format!("failed to render JSON: {err}"))),
            RenderFormat::Markdown => Ok(render_markdown(graph)),
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
        let nodes: Vec<_> = graph.nodes.iter().filter(|node| node.kind == kind).collect();
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

fn render_node(node: &AtlasNode, out: &mut String) {
    out.push_str(&format!("- `{}` — {}\n", node.id, node.title));
    out.push_str(&format!("  - Source: `{}`\n", node.provenance.source));
    if let Some(summary) = &node.summary {
        out.push_str(&format!("  - Summary: {}\n", summary));
    }
    if !node.paths.is_empty() {
        let joined = node
            .paths
            .iter()
            .map(|path| format!("`{}`", path.pattern))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  - Paths: {}\n", joined));
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
