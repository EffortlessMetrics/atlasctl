#![forbid(unsafe_code)]

use atlasctl_types::{AtlasGraph, ImpactEnvelope, WhyEnvelope};
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

#[cfg(test)]
mod tests {
    use super::{is_workspace_root, project_root, workspace_root_from};
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
}
