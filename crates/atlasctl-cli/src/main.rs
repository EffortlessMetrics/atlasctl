#![forbid(unsafe_code)]

use atlasctl_app::RenderPort;
use atlasctl_app::{
    AtlasService, BuildOptions, CheckOutcome, CompileOptions, ImpactOptions, ImpactSource,
    QueryOptions, TraceOptions, WhyOptions,
};
use atlasctl_discover_fs::{Codeowners, FsDiscovery, GitDiff};
use atlasctl_render::AtlasRenderer;
use atlasctl_types::{
    AtlasId, ChangedPath, ExitCode, NodeKind, QueryRequest, RenderFormat, RepoRelativePath,
    TraceDirection, TraceRequest, ValidationProfile, WhyRequest, WhySubject,
};
use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::process::exit;
use std::{fs, str::FromStr};

#[derive(Debug, Parser)]
#[command(name = "atlasctl")]
#[command(about = "Compile and inspect a repo behavior/proof atlas", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(InitArgs),
    Scaffold(ScaffoldArgs),
    Build(BuildArgs),
    Check(CheckArgs),
    Doctor(DoctorArgs),
    Impacted(ImpactedArgs),
    ReviewPacket(ReviewPacketArgs),
    Why(WhyArgs),
    Query(QueryArgs),
    Trace(TraceArgs),
    Export(ExportArgs),
}

#[derive(Debug, Clone, Args)]
struct ScaffoldArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(value_enum)]
    kind: ScaffoldKind,
    id: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScaffoldKind {
    Scenario,
    Artifact,
    Requirement,
}

#[derive(Debug, Clone, Args)]
struct InitArgs {
    #[arg(long, default_value = ".")]
    repo_root: camino::Utf8PathBuf,
}

#[derive(Debug, Clone, Args)]
struct CommonArgs {
    #[arg(long, default_value = ".")]
    repo_root: Utf8PathBuf,
    #[arg(long)]
    config: Option<Utf8PathBuf>,
    #[arg(long, value_enum, default_value_t = ProfileArg::Default)]
    profile: ProfileArg,
}

#[derive(Debug, Clone, Args)]
struct BuildArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long, default_value = ".atlas")]
    out_dir: Utf8PathBuf,
}

#[derive(Debug, Clone, Args)]
struct CheckArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long, value_enum, default_value_t = OutputArg::Text)]
    format: OutputArg,
}

#[derive(Debug, Clone, Args)]
struct DoctorArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long, value_enum, default_value_t = OutputArg::Text)]
    format: OutputArg,
}

#[derive(Debug, Clone, Args)]
struct ImpactedArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    head: Option<String>,
    #[arg(long, value_delimiter = ' ')]
    paths: Option<Vec<String>>,
    #[arg(long, value_enum, default_value_t = OutputArg::Text)]
    format: OutputArg,
}

#[derive(Debug, Clone, Args)]
struct ReviewPacketArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long)]
    base: Option<String>,
    #[arg(long)]
    head: Option<String>,
    #[arg(long, value_delimiter = ' ')]
    paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Args)]
struct WhyArgs {
    #[command(flatten)]
    common: CommonArgs,
    subject: String,
    #[arg(long)]
    path: bool,
    #[arg(long, value_enum, default_value_t = OutputArg::Text)]
    format: OutputArg,
}

#[derive(Debug, Clone, Args)]
struct QueryArgs {
    #[command(flatten)]
    common: CommonArgs,
    needle: String,
    #[arg(long)]
    kind: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct TraceArgs {
    #[command(flatten)]
    common: CommonArgs,
    start: String,
    #[arg(long, value_enum, default_value_t = DirectionArg::Both)]
    direction: DirectionArg,
    #[arg(long, default_value_t = 2)]
    max_depth: usize,
}

#[derive(Debug, Clone, Args)]
struct ExportArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long, value_enum)]
    format: FormatArg,
    #[arg(long)]
    out: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProfileArg {
    Default,
    Ci,
    Strict,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputArg {
    Text,
    Json,
    Markdown,
    GhSummary,
    ReviewPacket,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Markdown,
    GhSummary,
    ReviewPacket,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DirectionArg {
    Outgoing,
    Incoming,
    Both,
}

impl From<ProfileArg> for ValidationProfile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Default => ValidationProfile::Default,
            ProfileArg::Ci => ValidationProfile::Ci,
            ProfileArg::Strict => ValidationProfile::Strict,
        }
    }
}

impl From<OutputArg> for RenderFormat {
    fn from(arg: OutputArg) -> Self {
        match arg {
            OutputArg::Text | OutputArg::Markdown => RenderFormat::Markdown,
            OutputArg::Json => RenderFormat::Json,
            OutputArg::GhSummary => RenderFormat::GitHubSummary,
            OutputArg::ReviewPacket => RenderFormat::ReviewPacket,
        }
    }
}

impl From<FormatArg> for RenderFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Json => RenderFormat::Json,
            FormatArg::Markdown => RenderFormat::Markdown,
            FormatArg::GhSummary => RenderFormat::GitHubSummary,
            FormatArg::ReviewPacket => RenderFormat::ReviewPacket,
        }
    }
}

impl From<DirectionArg> for TraceDirection {
    fn from(value: DirectionArg) -> Self {
        match value {
            DirectionArg::Outgoing => TraceDirection::Outgoing,
            DirectionArg::Incoming => TraceDirection::Incoming,
            DirectionArg::Both => TraceDirection::Both,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let service = AtlasService::new(FsDiscovery, AtlasRenderer, GitDiff, Codeowners);

    let code = match run(cli, service) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::RuntimeError
        }
    };

    exit(code as i32);
}

fn run(
    cli: Cli,
    service: AtlasService<FsDiscovery, AtlasRenderer, GitDiff, Codeowners>,
) -> Result<ExitCode, String> {
    match cli.command {
        Command::Init(args) => {
            let config_path = args.repo_root.join("atlas.toml");
            if config_path.exists() {
                return Err(format!("`{config_path}` already exists"));
            }

            let repo_name = args.repo_root.file_name().unwrap_or("repo").to_string();

            let content = format!(
                r#"[repo]
name = "{repo_name}"

[discovery]
roots = ["."]
ignore = ["target", ".git", "node_modules"]

[profiles.default]
require_scenario_command = false
warnings_as_errors = false

[profiles.ci]
require_scenario_command = true
require_artifact_producer = true
warnings_as_errors = true
"#
            );

            fs::write(&config_path, content)
                .map_err(|err| format!("failed to write `{config_path}`: {err}"))?;

            println!("Initialized atlas in `{config_path}`");
            Ok(ExitCode::Ok)
        }
        Command::Scaffold(args) => {
            let atlas_dir = args.common.repo_root.join("atlas");
            if !atlas_dir.exists() {
                fs::create_dir_all(&atlas_dir)
                    .map_err(|err| format!("failed to create `atlas/` directory: {err}"))?;
            }

            let id = if args.id.contains(':') {
                args.id.clone()
            } else {
                let prefix = match args.kind {
                    ScaffoldKind::Scenario => "scen",
                    ScaffoldKind::Artifact => "artifact",
                    ScaffoldKind::Requirement => "req",
                };
                format!("{}:{}", prefix, args.id)
            };

            let file_name = format!("{}.atlas.yaml", args.id.replace(':', "-"));
            let scaffold_path = atlas_dir.join(file_name);
            if scaffold_path.exists() {
                return Err(format!("`{scaffold_path}` already exists"));
            }

            let content = match args.kind {
                ScaffoldKind::Scenario => format!(
                    r#"nodes:
  - id: {id}
    kind: scenario
    title: {id}
    summary: |
      Enter scenario summary here.
    touches:
      - "tests/*.rs"
edges:
  - from: {id}
    kind: exercises
    to: crate:TODO
  - from: {id}
    kind: runs_with
    to: cmd:TODO
"#
                ),
                ScaffoldKind::Artifact => format!(
                    r#"nodes:
  - id: {id}
    kind: artifact
    title: {id}
    summary: |
      Enter artifact summary here.
"#
                ),
                ScaffoldKind::Requirement => format!(
                    r#"nodes:
  - id: {id}
    kind: requirement
    title: {id}
    summary: |
      Enter requirement summary here.
"#
                ),
            };
            let kind_name = match args.kind {
                ScaffoldKind::Scenario => "scenario",
                ScaffoldKind::Artifact => "artifact",
                ScaffoldKind::Requirement => "requirement",
            };

            fs::write(&scaffold_path, content)
                .map_err(|err| format!("failed to write `{scaffold_path}`: {err}"))?;

            println!("Scaffolded {} in `{scaffold_path}`", kind_name);
            Ok(ExitCode::Ok)
        }
        Command::Build(args) => {
            let options = BuildOptions {
                compile: compile_options(&args.common),
                formats: vec![RenderFormat::Json, RenderFormat::Markdown],
            };

            let outcome = service
                .build(&options)
                .map_err(|err| format!("build failed: {err}"))?;

            fs::create_dir_all(&args.out_dir)
                .map_err(|err| format!("failed to create `{}`: {err}", args.out_dir))?;

            for (format, content) in &outcome.rendered {
                let path = args.out_dir.join(format.file_name());
                fs::write(&path, content)
                    .map_err(|err| format!("failed to write `{}`: {err}", path))?;
            }

            print_summary(&outcome.graph, outcome.has_errors);
            Ok(if outcome.has_errors {
                ExitCode::ValidationFailed
            } else {
                ExitCode::Ok
            })
        }
        Command::Check(args) => {
            let outcome = service
                .check(&compile_options(&args.common))
                .map_err(|err| format!("check failed: {err}"))?;

            match args.format {
                OutputArg::Text => print_check(&outcome),
                OutputArg::Json => {
                    let json = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::Json)
                        .map_err(|err| format!("failed to render JSON: {err}"))?;
                    println!("{json}");
                }
                OutputArg::Markdown => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::Markdown)
                        .map_err(|err| format!("failed to render Markdown: {err}"))?;
                    println!("{md}");
                }
                OutputArg::GhSummary => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::GitHubSummary)
                        .map_err(|err| format!("failed to render GitHub summary: {err}"))?;
                    println!("{md}");
                }
                OutputArg::ReviewPacket => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::ReviewPacket)
                        .map_err(|err| format!("failed to render review packet: {err}"))?;
                    println!("{md}");
                }
            }

            Ok(if outcome.has_errors {
                ExitCode::ValidationFailed
            } else {
                ExitCode::Ok
            })
        }
        Command::Doctor(args) => {
            let outcome = service
                .doctor(&compile_options(&args.common))
                .map_err(|err| format!("doctor failed: {err}"))?;

            match args.format {
                OutputArg::Text => print_check(&outcome),
                OutputArg::Json => {
                    let json = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::Json)
                        .map_err(|err| format!("failed to render JSON: {err}"))?;
                    println!("{json}");
                }
                OutputArg::Markdown => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::Markdown)
                        .map_err(|err| format!("failed to render Markdown: {err}"))?;
                    println!("{md}");
                }
                OutputArg::GhSummary => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::GitHubSummary)
                        .map_err(|err| format!("failed to render GitHub summary: {err}"))?;
                    println!("{md}");
                }
                OutputArg::ReviewPacket => {
                    let md = service
                        .renderer
                        .render(&outcome.graph, RenderFormat::ReviewPacket)
                        .map_err(|err| format!("failed to render review packet: {err}"))?;
                    println!("{md}");
                }
            }

            Ok(if outcome.has_errors {
                ExitCode::ValidationFailed
            } else {
                ExitCode::Ok
            })
        }
        Command::Impacted(args) => {
            let source = impact_source(args.paths, args.base, args.head);

            let outcome = service
                .impacted(&ImpactOptions {
                    compile: compile_options(&args.common),
                    request: source,
                })
                .map_err(|err| format!("impacted failed: {err}"))?;

            match args.format {
                OutputArg::Text => print_impacted(&outcome),
                OutputArg::Json => {
                    let json = service
                        .renderer
                        .render_impact(&outcome.response, RenderFormat::Json)
                        .map_err(|err| format!("failed to render JSON: {err}"))?;
                    println!("{json}");
                }
                OutputArg::Markdown => {
                    let md = service
                        .renderer
                        .render_impact(&outcome.response, RenderFormat::Markdown)
                        .map_err(|err| format!("failed to render Markdown: {err}"))?;
                    println!("{md}");
                }
                OutputArg::GhSummary => {
                    let md = service
                        .renderer
                        .render_impact(&outcome.response, RenderFormat::GitHubSummary)
                        .map_err(|err| format!("failed to render GitHub summary: {err}"))?;
                    println!("{md}");
                }
                OutputArg::ReviewPacket => {
                    let md = service
                        .renderer
                        .render_impact(&outcome.response, RenderFormat::ReviewPacket)
                        .map_err(|err| format!("failed to render review packet: {err}"))?;
                    println!("{md}");
                }
            }

            if outcome.has_uncovered_error {
                Ok(ExitCode::ValidationFailed)
            } else {
                Ok(ExitCode::Ok)
            }
        }
        Command::ReviewPacket(args) => {
            let source = impact_source(args.paths, args.base, args.head);

            let outcome = service
                .impacted(&ImpactOptions {
                    compile: compile_options(&args.common),
                    request: source,
                })
                .map_err(|err| format!("review-packet failed: {err}"))?;

            let md = service
                .renderer
                .render_impact(&outcome.response, RenderFormat::ReviewPacket)
                .map_err(|err| format!("failed to render review packet: {err}"))?;
            println!("{md}");

            if outcome.has_uncovered_error {
                Ok(ExitCode::ValidationFailed)
            } else {
                Ok(ExitCode::Ok)
            }
        }
        Command::Why(args) => {
            let subject = if args.path {
                WhySubject::Path(RepoRelativePath::new(args.subject))
            } else {
                WhySubject::Id(
                    AtlasId::parse(args.subject)
                        .map_err(|err| format!("invalid node id: {err}"))?,
                )
            };

            let outcome = service
                .why(&WhyOptions {
                    compile: compile_options(&args.common),
                    request: WhyRequest { subject },
                })
                .map_err(|err| format!("why failed: {err}"))?;

            match args.format {
                OutputArg::Text => print_why(&outcome),
                OutputArg::Json => {
                    let json = service
                        .renderer
                        .render_why(
                            outcome.response.as_ref().ok_or("no response")?,
                            atlasctl_types::RenderFormat::Json,
                        )
                        .map_err(|err| format!("failed to render JSON: {err}"))?;
                    println!("{json}");
                }
                OutputArg::Markdown => {
                    let md = service
                        .renderer
                        .render_why(
                            outcome.response.as_ref().ok_or("no response")?,
                            atlasctl_types::RenderFormat::Markdown,
                        )
                        .map_err(|err| format!("failed to render Markdown: {err}"))?;
                    println!("{md}");
                }
                OutputArg::GhSummary => {
                    let md = service
                        .renderer
                        .render_why(
                            outcome.response.as_ref().ok_or("no response")?,
                            atlasctl_types::RenderFormat::GitHubSummary,
                        )
                        .map_err(|err| format!("failed to render GitHub summary: {err}"))?;
                    println!("{md}");
                }
                OutputArg::ReviewPacket => {
                    let md = service
                        .renderer
                        .render_why(
                            outcome.response.as_ref().ok_or("no response")?,
                            atlasctl_types::RenderFormat::ReviewPacket,
                        )
                        .map_err(|err| format!("failed to render review packet: {err}"))?;
                    println!("{md}");
                }
            }

            Ok(ExitCode::Ok)
        }
        Command::Query(args) => {
            let kind = match args.kind {
                Some(kind) => Some(
                    NodeKind::from_str(&kind).map_err(|_| format!("unknown node kind `{kind}`"))?,
                ),
                None => None,
            };

            let outcome = service
                .query(&QueryOptions {
                    compile: compile_options(&args.common),
                    request: QueryRequest {
                        needle: args.needle,
                        kind,
                    },
                })
                .map_err(|err| format!("query failed: {err}"))?;

            if outcome.response.matches.is_empty() {
                println!("no matches");
            } else {
                for hit in outcome.response.matches {
                    println!("{} [{}] {}", hit.node.id, hit.node.kind, hit.node.title);
                    println!("  score: {}", hit.score);
                    println!("  source: {}", hit.node.provenance.source);
                    if let Some(summary) = hit.node.summary {
                        println!("  summary: {}", summary);
                    }
                }
            }

            Ok(ExitCode::Ok)
        }
        Command::Trace(args) => {
            let start = args
                .start
                .parse()
                .map_err(|err| format!("invalid trace root: {err}"))?;

            let outcome = service
                .trace(&TraceOptions {
                    compile: compile_options(&args.common),
                    request: TraceRequest {
                        start,
                        direction: args.direction.into(),
                        max_depth: args.max_depth,
                    },
                })
                .map_err(|err| format!("trace failed: {err}"))?;

            let Some(response) = outcome.response else {
                println!("trace root not found");
                return Ok(ExitCode::ValidationFailed);
            };

            println!(
                "root: {} [{}] {}",
                response.root.id, response.root.kind, response.root.title
            );
            println!();

            if response.nodes.is_empty() {
                println!("no linked nodes");
            } else {
                println!("nodes:");
                for node in response.nodes {
                    println!("- {} [{}] {}", node.id, node.kind, node.title);
                }
            }

            if !response.edges.is_empty() {
                println!();
                println!("edges:");
                for trace_edge in response.edges {
                    println!(
                        "- depth {}: {} --{}--> {}",
                        trace_edge.depth,
                        trace_edge.edge.from,
                        trace_edge.edge.kind,
                        trace_edge.edge.to
                    );
                }
            }

            Ok(ExitCode::Ok)
        }
        Command::Export(args) => {
            let format: RenderFormat = args.format.into();
            let outcome = service
                .build(&BuildOptions {
                    compile: compile_options(&args.common),
                    formats: vec![format],
                })
                .map_err(|err| format!("export failed: {err}"))?;

            let rendered = outcome
                .rendered
                .get(&format)
                .ok_or_else(|| "rendered output missing".to_string())?;

            if let Some(path) = args.out {
                if let Some(parent) = path.parent()
                    && !parent.as_str().is_empty()
                {
                    fs::create_dir_all(parent)
                        .map_err(|err| format!("failed to create `{}`: {err}", parent))?;
                }
                fs::write(&path, rendered)
                    .map_err(|err| format!("failed to write `{}`: {err}", path))?;
            } else {
                println!("{rendered}");
            }

            Ok(if outcome.has_errors {
                ExitCode::ValidationFailed
            } else {
                ExitCode::Ok
            })
        }
    }
}

fn compile_options(common: &CommonArgs) -> CompileOptions {
    CompileOptions {
        repo_root: common.repo_root.clone(),
        config_path: common.config.clone(),
        profile: common.profile.into(),
    }
}

fn impact_source(
    paths: Option<Vec<String>>,
    base: Option<String>,
    head: Option<String>,
) -> ImpactSource {
    if let Some(paths) = paths {
        ImpactSource::Paths(
            paths
                .into_iter()
                .map(|p| ChangedPath {
                    path: RepoRelativePath::new(p),
                })
                .collect(),
        )
    } else {
        ImpactSource::Diff {
            base: base.unwrap_or_else(|| "main".to_string()),
            head: head.unwrap_or_else(|| "HEAD".to_string()),
        }
    }
}

fn print_summary(graph: &atlasctl_types::AtlasGraph, has_errors: bool) {
    println!("repo: {}", graph.repo.name);
    println!(
        "nodes: {}  edges: {}  diagnostics: {}",
        graph.metrics.node_count, graph.metrics.edge_count, graph.metrics.diagnostic_count
    );
    println!(
        "errors: {}  warnings: {}",
        graph.metrics.error_count, graph.metrics.warning_count
    );

    if has_errors {
        println!("status: invalid");
    } else {
        println!("status: ok");
    }
}

fn print_check(outcome: &CheckOutcome) {
    print_summary(&outcome.graph, outcome.has_errors);
    if outcome.graph.diagnostics.is_empty() {
        return;
    }

    println!();
    for diagnostic in &outcome.graph.diagnostics {
        println!(
            "[{}] {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
        if let Some(subject) = &diagnostic.subject {
            println!("  subject: {}", subject);
        }
        if let Some(location) = &diagnostic.location {
            println!("  location: {}", location.path);
        }
    }
}

fn print_impacted(outcome: &atlasctl_app::ImpactOutcome) {
    println!("Impact Analysis:");
    println!("  impacted nodes: {}", outcome.response.impacted.len());
    println!("  uncovered changes: {}", outcome.response.uncovered.len());
    if outcome.has_uncovered_warning {
        println!("  status: warnings (uncovered changes)");
    } else if outcome.has_uncovered_error {
        println!("  status: errors (uncovered changes)");
    } else {
        println!("  status: ok");
    }

    if !outcome.response.impacted.is_empty() {
        println!("\nImpacted Nodes:");
        for hit in &outcome.response.impacted {
            println!("- {} ({}) — {}", hit.node.id, hit.node.kind, hit.node.title);
            println!("  reason: {}", hit.reason);
            if !hit.owners.is_empty() {
                println!("  owners: {}", hit.owners.join(", "));
            }
        }
    }

    if !outcome.response.uncovered.is_empty() {
        println!("\nUncovered Changes:");
        for path in &outcome.response.uncovered {
            println!("- {}", path.path);
        }
    }
}

fn print_why(outcome: &atlasctl_app::WhyOutcome) {
    let Some(response) = &outcome.response else {
        println!("No matching node found.");
        return;
    };

    println!("Node: {} ({})", response.root.id, response.root.kind);
    println!("Title: {}", response.root.title);
    if let Some(summary) = &response.root.summary {
        println!("Summary: {}", summary);
    }
    println!("Source: {}", response.root.provenance.source);

    if !response.root.owns.is_empty() {
        println!("Owns:");
        for p in &response.root.owns {
            println!("  - {}", p.pattern);
        }
    }

    if !response.root.touches.is_empty() {
        println!("Touches:");
        for p in &response.root.touches {
            println!("  - {}", p.pattern);
        }
    }

    if response.chain.is_empty() {
        println!("\nNo immediate proof chain found.");
    } else {
        println!("\nProof chain:");
        for step in &response.chain {
            let direction = match step.direction {
                atlasctl_types::TraceDirection::Incoming => "<--",
                atlasctl_types::TraceDirection::Outgoing => "-->",
                atlasctl_types::TraceDirection::Both => "<->",
            };
            println!(
                "  {} [{}] {} ({})",
                direction, step.relationship, step.node.id, step.node.kind
            );
        }
    }
}
