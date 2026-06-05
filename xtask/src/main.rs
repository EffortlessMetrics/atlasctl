#![forbid(unsafe_code)]

use atlasctl_types::{AtlasGraph, ImpactEnvelope, ImpactMetrics, WhyEnvelope};
use camino::Utf8PathBuf;
use schemars::schema_for;
use std::env;
use std::fs;
use std::process::{Command, exit};

fn main() {
    let mut args = env::args().skip(1);
    let Some(task) = args.next() else {
        eprintln!(
            "usage: cargo run -p xtask -- <ci-fast|ci-full|smoke|docs-check|release-check|schema|scorecard>"
        );
        exit(2);
    };

    let result = match task.as_str() {
        "ci-fast" => ci_fast(),
        "ci-full" => ci_full(),
        "smoke" => smoke(),
        "golden" => golden(),
        "mutants" => mutants(),
        "docs-check" => docs_check(),
        "release-check" => release_check(),
        "schema" => {
            let check = args.next().map(|s| s == "--check").unwrap_or(false);
            schema(check)
        }
        "scorecard" => scorecard(args.collect()),
        other => Err(format!("unknown task `{other}`")),
    };

    match result {
        Ok(()) => {}
        Err(message) => {
            eprintln!("xtask error: {message}");
            exit(1);
        }
    }
}

fn ci_fast() -> Result<(), String> {
    run("cargo", &["fmt", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace"])?;
    Ok(())
}

fn ci_full() -> Result<(), String> {
    ci_fast()?;
    smoke()?;
    docs_check()?;
    schema(true)?;
    Ok(())
}

fn smoke() -> Result<(), String> {
    run(
        "cargo",
        &[
            "run",
            "-p",
            "atlasctl-cli",
            "--",
            "build",
            "--out-dir",
            ".atlas-smoke",
        ],
    )?;
    Ok(())
}

fn golden() -> Result<(), String> {
    run("cargo", &["insta", "test", "--accept"])
}

fn mutants() -> Result<(), String> {
    run("cargo", &["mutants", "-d", "crates/atlasctl-core"])
}

fn docs_check() -> Result<(), String> {
    run(
        "cargo",
        &[
            "run",
            "-p",
            "atlasctl-cli",
            "--",
            "check",
            "--profile",
            "ci",
        ],
    )?;
    Ok(())
}

fn release_check() -> Result<(), String> {
    ci_full()?;
    mutants()?;
    Ok(())
}

fn schema(check: bool) -> Result<(), String> {
    let project_root = project_root();
    let schema_dir = project_root.join("schemas");
    if !schema_dir.exists() {
        fs::create_dir_all(&schema_dir).map_err(|e| e.to_string())?;
    }

    let targets = vec![
        ("atlas.schema.json", schema_for!(AtlasGraph)),
        ("impact.schema.json", schema_for!(ImpactEnvelope)),
        ("why.schema.json", schema_for!(WhyEnvelope)),
        // For now doctor result is the full graph
        ("doctor.schema.json", schema_for!(AtlasGraph)),
    ];

    for (file_name, schema) in targets {
        let path = schema_dir.join(file_name);
        let content = serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())? + "\n";

        if check {
            if !path.exists() {
                return Err(format!("Schema file `{file_name}` is missing"));
            }
            let existing = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if existing != content {
                return Err(format!(
                    "Schema file `{file_name}` is out of date. Run `cargo run -p xtask -- schema` to update."
                ));
            }
        } else {
            fs::write(&path, content).map_err(|e| e.to_string())?;
            println!("Updated schema `{file_name}`");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScorecardArgs {
    repo_root: Utf8PathBuf,
    base: String,
    head: String,
    sample: String,
    out: Option<Utf8PathBuf>,
    include_header: bool,
}

impl Default for ScorecardArgs {
    fn default() -> Self {
        Self {
            repo_root: Utf8PathBuf::from("."),
            base: "main".to_string(),
            head: "HEAD".to_string(),
            sample: "Local sample".to_string(),
            out: None,
            include_header: false,
        }
    }
}

fn scorecard(raw_args: Vec<String>) -> Result<(), String> {
    let args = parse_scorecard_args(&raw_args)?;
    let envelope = scorecard_impact_envelope(&args)?;
    let row = format_scorecard_markdown(&args, &envelope.payload.metrics);

    if let Some(path) = &args.out {
        write_scorecard_output(path, &row)?;
    } else {
        println!("{row}");
    }

    Ok(())
}

fn parse_scorecard_args(raw_args: &[String]) -> Result<ScorecardArgs, String> {
    let mut parsed = ScorecardArgs::default();
    let mut index = 0;

    while index < raw_args.len() {
        match raw_args[index].as_str() {
            "--repo-root" => {
                parsed.repo_root =
                    Utf8PathBuf::from(next_scorecard_value(raw_args, &mut index, "--repo-root")?);
            }
            "--base" => {
                parsed.base = next_scorecard_value(raw_args, &mut index, "--base")?;
            }
            "--head" => {
                parsed.head = next_scorecard_value(raw_args, &mut index, "--head")?;
            }
            "--sample" => {
                parsed.sample = next_scorecard_value(raw_args, &mut index, "--sample")?;
            }
            "--out" => {
                parsed.out = Some(Utf8PathBuf::from(next_scorecard_value(
                    raw_args, &mut index, "--out",
                )?));
            }
            "--header" => {
                parsed.include_header = true;
            }
            "-h" | "--help" => {
                return Err(scorecard_usage());
            }
            other => {
                return Err(format!(
                    "unknown scorecard argument `{other}`\n{}",
                    scorecard_usage()
                ));
            }
        }

        index += 1;
    }

    Ok(parsed)
}

fn next_scorecard_value(
    raw_args: &[String],
    index: &mut usize,
    flag: &str,
) -> Result<String, String> {
    *index += 1;
    raw_args
        .get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for `{flag}`\n{}", scorecard_usage()))
}

fn scorecard_usage() -> String {
    "usage: cargo run -p xtask -- scorecard [--repo-root <path>] [--base <rev>] [--head <rev>] [--sample <label>] [--out <path>] [--header]".to_string()
}

fn scorecard_impact_envelope(args: &ScorecardArgs) -> Result<ImpactEnvelope, String> {
    let command_args = vec![
        "run".to_string(),
        "-p".to_string(),
        "atlasctl-cli".to_string(),
        "--".to_string(),
        "impacted".to_string(),
        "--repo-root".to_string(),
        args.repo_root.to_string(),
        "--base".to_string(),
        args.base.clone(),
        "--head".to_string(),
        args.head.clone(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let output = run_output("cargo", &command_args)?;
    serde_json::from_str(&output).map_err(|err| {
        format!(
            "failed to parse impacted JSON output for `{}`: {err}",
            args.sample
        )
    })
}

fn format_scorecard_markdown(args: &ScorecardArgs, metrics: &ImpactMetrics) -> String {
    let changed = metrics.changed_path_count;
    let uncovered = metrics.uncovered_path_count;
    let covered = changed.saturating_sub(uncovered);
    let uncovered_rate = percentage(uncovered, changed);
    let range = format!(
        "`{}..{}`",
        markdown_cell(&args.base),
        markdown_cell(&args.head)
    );
    let row = format!(
        "| {} | {} | {} | {} | {} | {:.1}% | {} | {} | {} | {} | {} | {}% |",
        markdown_cell(&args.sample),
        range,
        changed,
        covered,
        uncovered,
        uncovered_rate,
        metrics.impacted_node_count,
        metrics.missing_evidence_count,
        metrics.scope_warning_count,
        metrics.touched_only_path_count,
        metrics.multi_owner_path_count,
        metrics.coverage_percent
    );

    if args.include_header {
        format!(
            "{}\n{}\n{}",
            scorecard_markdown_header(),
            scorecard_markdown_separator(),
            row
        )
    } else {
        row
    }
}

fn scorecard_markdown_header() -> &'static str {
    "| Sample | Base..Head | Changed Paths | Covered | Uncovered | Uncovered Rate | Impacted Nodes | Missing Evidence | Scope Warnings | Touched-Only Paths | Multi-Owner Paths | Coverage |"
}

fn scorecard_markdown_separator() -> &'static str {
    "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
}

fn percentage(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn markdown_cell(value: &str) -> String {
    value.replace('\n', " ").replace('|', "\\|")
}

fn write_scorecard_output(path: &Utf8PathBuf, content: &str) -> Result<(), String> {
    let path = if path.is_absolute() {
        path.clone()
    } else {
        project_root().join(path)
    };

    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| format!("failed to create `{parent}`: {err}"))?;
    }

    fs::write(&path, format!("{content}\n"))
        .map_err(|err| format!("failed to write `{path}`: {err}"))
}

fn project_root() -> Utf8PathBuf {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let manifest_dir = Utf8PathBuf::from(manifest_dir);
        if let Some(parent) = manifest_dir.parent() {
            return parent.to_path_buf();
        }
    }

    if let Some(project_root) = project_root_from_current_exe() {
        return project_root;
    }

    if let Some(project_root) = project_root_from_current_dir() {
        return project_root;
    }

    env::current_dir()
        .ok()
        .and_then(|dir| Utf8PathBuf::from_path_buf(dir).ok())
        .unwrap_or_else(|| Utf8PathBuf::from("."))
}

fn project_root_from_current_exe() -> Option<Utf8PathBuf> {
    let current_exe = env::current_exe().ok()?;
    workspace_root_from(Utf8PathBuf::from_path_buf(current_exe.parent()?.to_path_buf()).ok()?)
}

fn project_root_from_current_dir() -> Option<Utf8PathBuf> {
    let current_dir = env::current_dir().ok()?;
    workspace_root_from(Utf8PathBuf::from_path_buf(current_dir).ok()?)
}

fn workspace_root_from(mut start: Utf8PathBuf) -> Option<Utf8PathBuf> {
    loop {
        if is_workspace_root(&start) {
            return Some(start);
        }

        if !start.pop() {
            return None;
        }
    }
}

fn is_workspace_root(path: &camino::Utf8Path) -> bool {
    let manifest_path = path.join("Cargo.toml");
    let manifest = match std::fs::read_to_string(manifest_path) {
        Ok(contents) => contents,
        Err(_) => return false,
    };

    manifest.contains("[workspace]")
}

fn run(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run `{command}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{command} {}` exited with {status}",
            args.join(" ")
        ))
    }
}

fn run_output(command: &str, args: &[String]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run `{command}`: {err}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`{command} {}` exited with {}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            output.status,
            stdout.trim_end(),
            stderr.trim_end()
        ));
    }

    String::from_utf8(output.stdout).map_err(|err| {
        format!(
            "`{command} {}` emitted non-UTF-8 output: {err}",
            args.join(" ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ScorecardArgs, format_scorecard_markdown, is_workspace_root, parse_scorecard_args,
        project_root, workspace_root_from,
    };
    use atlasctl_types::ImpactMetrics;
    use std::{
        env, fs,
        path::PathBuf,
        process::id,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be available")
            .as_nanos();
        env::temp_dir().join(format!("atlasctl-xtask-{prefix}-{nanos}-{}", id()))
    }

    #[test]
    fn project_root_points_to_workspace_root() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let expected_root = manifest_dir
            .parent()
            .and_then(|path| camino::Utf8PathBuf::from_path_buf(path.to_path_buf()).ok())
            .expect("xtask manifest should be inside the workspace");

        assert_eq!(project_root(), expected_root);
    }

    #[test]
    fn workspace_root_from_current_exe_matches_workspace_root() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|path| camino::Utf8PathBuf::from_path_buf(path.to_path_buf()).ok())
            .expect("xtask manifest should be inside the workspace");

        let start = env::current_exe()
            .expect("current_exe should be available")
            .parent()
            .map(|path| path.to_path_buf())
            .and_then(|path| camino::Utf8PathBuf::from_path_buf(path).ok())
            .expect("current executable path should be utf8");

        assert_eq!(workspace_root_from(start), Some(workspace_root));
    }

    #[test]
    fn is_workspace_root_is_false_for_non_workspace_manifest() {
        let dir = unique_temp_dir("no-workspace");
        fs::create_dir_all(&dir).expect("test temp directory should exist");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"tmp\"")
            .expect("non-workspace manifest should be written");

        let start = camino::Utf8PathBuf::from_path_buf(dir).expect("temp path should be utf8");
        assert!(!is_workspace_root(&start));
        fs::remove_dir_all(start.as_std_path()).expect("temp dir should be cleaned up");
    }

    #[test]
    fn workspace_root_from_temp_path_without_workspace_is_none() {
        let dir = unique_temp_dir("workspace-root-missing");
        fs::create_dir_all(&dir).expect("test temp directory should exist");

        let start =
            camino::Utf8PathBuf::from_path_buf(dir.clone()).expect("temp path should be utf8");
        assert!(workspace_root_from(start).is_none());
        fs::remove_dir_all(&dir).expect("temp dir should be cleaned up");
    }

    #[test]
    fn parse_scorecard_args_accepts_local_defaults_and_overrides() {
        let args = vec![
            "--repo-root".to_string(),
            "H:/Code/Rust/atlasctl".to_string(),
            "--base".to_string(),
            "abc123".to_string(),
            "--head".to_string(),
            "def456".to_string(),
            "--sample".to_string(),
            "PR #42".to_string(),
            "--out".to_string(),
            "target/scorecard.md".to_string(),
            "--header".to_string(),
        ];

        let parsed = parse_scorecard_args(&args).expect("scorecard args should parse");

        assert_eq!(
            parsed.repo_root,
            camino::Utf8PathBuf::from("H:/Code/Rust/atlasctl")
        );
        assert_eq!(parsed.base, "abc123");
        assert_eq!(parsed.head, "def456");
        assert_eq!(parsed.sample, "PR #42");
        assert_eq!(
            parsed.out,
            Some(camino::Utf8PathBuf::from("target/scorecard.md"))
        );
        assert!(parsed.include_header);
    }

    #[test]
    fn format_scorecard_markdown_uses_impact_metrics() {
        let args = ScorecardArgs {
            base: "base".to_string(),
            head: "head".to_string(),
            sample: "sample | one".to_string(),
            include_header: true,
            ..ScorecardArgs::default()
        };
        let metrics = ImpactMetrics {
            changed_path_count: 4,
            uncovered_path_count: 1,
            impacted_node_count: 7,
            missing_evidence_count: 2,
            scope_warning_count: 3,
            touched_only_path_count: 1,
            multi_owner_path_count: 1,
            coverage_percent: 75,
            ..ImpactMetrics::default()
        };

        let row = format_scorecard_markdown(&args, &metrics);

        assert!(row.contains("| Sample | Base..Head |"));
        assert!(row.contains(
            "| sample \\| one | `base..head` | 4 | 3 | 1 | 25.0% | 7 | 2 | 3 | 1 | 1 | 75% |"
        ));
    }
}
