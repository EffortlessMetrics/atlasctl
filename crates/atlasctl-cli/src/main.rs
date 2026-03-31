#![forbid(unsafe_code)]

use atlasctl_app::{
    AtlasService, BuildOptions, CheckOutcome, CompileOptions, QueryOptions, TraceOptions,
};
use atlasctl_codes::ExitCode;
use atlasctl_discover_fs::FsDiscovery;
use atlasctl_render::AtlasRenderer;
use atlasctl_types::{
    NodeKind, QueryRequest, RenderFormat, TraceDirection, TraceRequest, ValidationProfile,
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
    Build(BuildArgs),
    Check(CheckArgs),
    Query(QueryArgs),
    Trace(TraceArgs),
    Export(ExportArgs),
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
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Markdown,
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

impl From<FormatArg> for RenderFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Json => RenderFormat::Json,
            FormatArg::Markdown => RenderFormat::Markdown,
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
    let service = AtlasService::new(FsDiscovery, AtlasRenderer);

    let code = match run(cli, service) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::RuntimeError
        }
    };

    exit(code as i32);
}

fn run(cli: Cli, service: AtlasService<FsDiscovery, AtlasRenderer>) -> Result<ExitCode, String> {
    match cli.command {
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
                    let json = serde_json::to_string_pretty(&outcome.graph)
                        .map_err(|err| format!("failed to serialize graph: {err}"))?;
                    println!("{json}");
                }
            }

            Ok(if outcome.has_errors {
                ExitCode::ValidationFailed
            } else {
                ExitCode::Ok
            })
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
                if let Some(parent) = path.parent() {
                    if !parent.as_str().is_empty() {
                        fs::create_dir_all(parent)
                            .map_err(|err| format!("failed to create `{}`: {err}", parent))?;
                    }
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
