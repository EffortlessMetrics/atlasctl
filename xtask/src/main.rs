#![forbid(unsafe_code)]

use atlasctl_types::{AtlasGraph, ImpactResponse, WhyResponse};
use camino::Utf8PathBuf;
use schemars::schema_for;
use std::env;
use std::fs;
use std::process::{Command, exit};

fn main() {
    let mut args = env::args().skip(1);
    let Some(task) = args.next() else {
        eprintln!(
            "usage: cargo run -p xtask -- <ci-fast|ci-full|smoke|docs-check|release-check|schema>"
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
        ("impact.schema.json", schema_for!(ImpactResponse)),
        ("why.schema.json", schema_for!(WhyResponse)),
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

fn project_root() -> Utf8PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    Utf8PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .to_path_buf()
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
