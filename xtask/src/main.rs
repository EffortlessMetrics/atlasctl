#![forbid(unsafe_code)]

use std::env;
use std::process::{exit, Command};

fn main() {
    let mut args = env::args().skip(1);
    let Some(task) = args.next() else {
        eprintln!("usage: cargo run -p xtask -- <ci-fast|ci-full|smoke|docs-check|release-check>");
        exit(2);
    };

    let result = match task.as_str() {
        "ci-fast" => ci_fast(),
        "ci-full" => ci_full(),
        "smoke" => smoke(),
        "docs-check" => docs_check(),
        "release-check" => release_check(),
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
    ci_full()
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
