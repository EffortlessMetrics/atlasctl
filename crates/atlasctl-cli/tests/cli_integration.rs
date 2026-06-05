// Integration tests for atlasctl CLI commands
// These tests verify the CLI behavior against fixture repos

use assert_cmd::Command;
use atlasctl_types::{AtlasGraph, DiagnosticCode, ImpactEnvelope, WhyEnvelope};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::TempDir;

// Helper to get path to a fixture repo
fn fixture_path(name: &str) -> std::path::PathBuf {
    // Get the workspace directory from CARGO_MANIFEST_DIR or use a default
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    // Navigate from crate dir to workspace root (crates/atlasctl-cli -> ..)
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find workspace root");

    workspace_root.join("fixtures/repos").join(name)
}

// Helper to create a temp dir and copy a fixture into it
fn setup_temp_fixture(fixture_name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let fixture_src = fixture_path(fixture_name);

    // Copy fixture contents to temp dir
    copy_dir_recursive(&fixture_src, temp_dir.path()).expect("Failed to copy fixture");

    temp_dir
}

// Recursive directory copy helper
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git command should execute");
    assert!(
        output.status.success(),
        "git command failed: git {}\\nstdout: {}\\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_with_config(dir: &Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git command should execute");
    assert!(
        status.success(),
        "git command failed: git {}",
        args.join(" ")
    );
}

fn assert_repo_relative_path(path: &str, field: &str) {
    let has_windows_drive = path.len() >= 3
        && path.as_bytes()[1] == b':'
        && (path.as_bytes()[2] == b'\\' || path.as_bytes()[2] == b'/')
        && path.as_bytes()[0].is_ascii_alphabetic();

    assert!(
        !path.starts_with('/'),
        "{} is absolute (unix-style): {path}",
        field
    );
    assert!(
        !has_windows_drive,
        "{} is absolute (windows-style): {path}",
        field
    );
    assert!(
        !path.contains('\\'),
        "{} uses backslash path separator: {path}",
        field
    );
}

fn assert_repo_relative_source(path: &str, field: &str) {
    assert_repo_relative_path(path, field);
    assert!(
        !path.contains(".."),
        "{} is parent-relative (should be normalized): {path}",
        field
    );
}

fn setup_temp_git_fixture() -> (TempDir, String, String) {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let repo_root = temp_dir.path();

    git_with_config(
        repo_root,
        &[
            "-c",
            "user.name=atlasctl-ci",
            "-c",
            "user.email=atlasctl-ci@example.com",
            "init",
        ],
    );
    git_with_config(
        repo_root,
        &[
            "-c",
            "user.name=atlasctl-ci",
            "-c",
            "user.email=atlasctl-ci@example.com",
            "add",
            ".",
        ],
    );
    git_with_config(
        repo_root,
        &[
            "-c",
            "user.name=atlasctl-ci",
            "-c",
            "user.email=atlasctl-ci@example.com",
            "commit",
            "-m",
            "seed base snapshot",
        ],
    );
    let base = git(repo_root, &["rev-parse", "HEAD"]);

    let engine_file = repo_root.join("crates/engine/src/lib.rs");
    fs::write(
        &engine_file,
        fs::read_to_string(&engine_file).unwrap() + "\n// atlasctl fixture edit\n",
    )
    .unwrap();
    git_with_config(
        repo_root,
        &[
            "-c",
            "user.name=atlasctl-ci",
            "-c",
            "user.email=atlasctl-ci@example.com",
            "add",
            "crates/engine/src/lib.rs",
        ],
    );
    git_with_config(
        repo_root,
        &[
            "-c",
            "user.name=atlasctl-ci",
            "-c",
            "user.email=atlasctl-ci@example.com",
            "commit",
            "-m",
            "touch engine source",
        ],
    );
    let head = git(repo_root, &["rev-parse", "HEAD"]);

    (temp_dir, base, head)
}

// ============================================================================
// BUILD COMMAND TESTS
// ============================================================================

#[test]
fn test_build_valid_repo() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_dir = temp_dir.path().join(".atlas");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "build",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--out-dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: ok"))
        .stdout(predicate::str::contains("nodes: 6"))
        .stdout(predicate::str::contains("edges: 5"));

    // Verify output files were created
    assert!(output_dir.join("atlas.json").exists());
    assert!(output_dir.join("atlas.md").exists());
}

#[test]
fn test_build_with_custom_output_dir() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_dir = temp_dir.path().join("custom-output");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "build",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--out-dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_dir.join("atlas.json").exists());
    assert!(output_dir.join("atlas.md").exists());
}

#[test]
fn test_build_with_ci_profile() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "build",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "ci",
        ])
        .assert()
        .success();
}

#[test]
fn test_build_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let output_dir = temp_dir.path().join("custom-build-output");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "build",
            "--repo-root",
            "..",
            "--config",
            "atlasctl-custom.toml",
            "--out-dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_dir.join("atlas.json").exists());
    assert!(output_dir.join("atlas.md").exists());
}

#[test]
fn test_build_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    let output_dir = temp_dir.path().join("custom-build-output");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "build",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "--out-dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_dir.join("atlas.json").exists());
    assert!(output_dir.join("atlas.md").exists());
}

#[test]
fn test_build_with_strict_profile() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "build",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "strict",
        ])
        .assert()
        .success();
}

#[test]
fn test_build_broken_link_repo() {
    let temp_dir = setup_temp_fixture("broken-link");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("status: invalid"))
        .stdout(predicate::str::contains("errors:"));
}

#[test]
fn test_build_duplicate_id_repo() {
    let temp_dir = setup_temp_fixture("duplicate-id");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("status: invalid"))
        .stdout(predicate::str::contains("errors:"));
}

#[test]
fn test_build_orphan_scenario_repo() {
    let temp_dir = setup_temp_fixture("orphan-scenario");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("status: invalid"))
        .stdout(predicate::str::contains("errors:"));
}

#[test]
fn test_build_nonexistent_repo() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", "/nonexistent/path"])
        .assert()
        .code(4) // Exit code 4 for runtime error
        .stderr(predicate::str::contains("error"));
}

// ============================================================================
// CHECK COMMAND TESTS
// ============================================================================

#[test]
fn test_check_valid_repo_text_format() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: ok"))
        .stdout(predicate::str::contains("errors: 0"))
        .stdout(predicate::str::contains("warnings: 0"));
}

#[test]
fn test_check_valid_repo_json_format() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "check",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"repo\""))
        .stdout(predicate::str::contains("\"metrics\""));
}

#[test]
fn test_check_broken_link_repo() {
    let temp_dir = setup_temp_fixture("broken-link");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("status: invalid"))
        .stdout(predicate::str::contains("errors:"));
}

#[test]
fn test_check_with_ci_profile() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "check",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "ci",
        ])
        .assert()
        .success();
}

#[test]
fn test_check_with_strict_profile() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "check",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "strict",
        ])
        .assert()
        .success();
}

#[test]
fn test_check_json_outputs_repo_relative_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "check",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--profile",
            "ci",
        ])
        .output()
        .expect("check json should execute");

    assert!(output.status.success());
    let graph: AtlasGraph =
        serde_json::from_slice(&output.stdout).expect("check json should parse");
    assert_eq!(graph.schema_version, 1);

    for node in graph.nodes {
        assert_repo_relative_source(
            node.provenance.source.as_str(),
            "check json node provenance source",
        );
    }

    for diagnostic in graph.diagnostics {
        if let Some(location) = diagnostic.location {
            assert_repo_relative_path(
                location.path.as_str(),
                "check json diagnostic location path",
            );
        }
    }
}

#[test]
fn test_check_with_parent_relative_repo_root_is_resolved_from_cwd() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args(["check", "--repo-root", ".."])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: ok"));
}

#[test]
fn test_check_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("doctor-drift");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("strict-default.toml");
    fs::write(
        &config_path,
        "[profiles.default]\nwarnings_as_errors = true\n",
    )
    .unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "check",
            "--repo-root",
            "..",
            "--config",
            "strict-default.toml",
            "--profile",
            "default",
        ])
        .output()
        .expect("check command should execute");

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status: invalid"));
}

#[test]
fn test_check_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("doctor-drift");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("strict-default.toml");
    fs::write(
        &config_path,
        "[profiles.default]\nwarnings_as_errors = true\n",
    )
    .unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "check",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "strict-default.toml",
            "--profile",
            "default",
        ])
        .output()
        .expect("check command should execute");

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status: invalid"));
}

#[test]
fn test_review_packet_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "review-packet",
            "--repo-root",
            "..",
            "--config",
            "atlasctl-custom.toml",
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(value["schema_version"], 1);
    let impacted = value["payload"]["impacted"]
        .as_array()
        .expect("impacted should be an array");
    assert!(impacted.is_empty());
    let uncovered = value["payload"]["uncovered"]
        .as_array()
        .expect("uncovered should be an array");
    assert_eq!(uncovered.len(), 1);
}

#[test]
fn test_review_packet_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "review-packet",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(value["schema_version"], 1);
    let impacted = value["payload"]["impacted"]
        .as_array()
        .expect("impacted should be an array");
    assert!(impacted.is_empty());
    let uncovered = value["payload"]["uncovered"]
        .as_array()
        .expect("uncovered should be an array");
    assert_eq!(uncovered.len(), 1);
}

// ============================================================================
// DOCTOR COMMAND TESTS
// ============================================================================

#[test]
fn test_doctor_drift_repo() {
    let temp_dir = setup_temp_fixture("doctor-drift");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["doctor", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success() // Warnings don't cause failure in default profile
        .stdout(predicate::str::contains("status: ok"))
        .stdout(predicate::str::contains("warnings: 4"))
        .stdout(predicate::str::contains("diagnostics: 5"));
}

#[test]
fn test_doctor_json_format() {
    let temp_dir = setup_temp_fixture("doctor-drift");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "doctor",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"diagnostics\""))
        .stdout(predicate::str::contains("orphan_node"));
}

#[test]
fn test_doctor_json_paths_are_repo_relative() {
    let temp_dir = setup_temp_fixture("doctor-drift");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "doctor",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("doctor json should execute");

    assert!(output.status.success());
    let graph: AtlasGraph =
        serde_json::from_slice(&output.stdout).expect("doctor json should parse");
    assert_eq!(graph.schema_version, 1);

    for node in graph.nodes {
        assert_repo_relative_source(node.provenance.source.as_str(), "node provenance source");
    }

    for diagnostic in graph.diagnostics {
        if let Some(location) = diagnostic.location {
            assert_repo_relative_path(location.path.as_str(), "diagnostic location path");
        }
    }
}

#[test]
fn test_doctor_with_ci_profile_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-ci.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "doctor",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-ci.toml",
            "--profile",
            "ci",
            "--format",
            "json",
        ])
        .output()
        .expect("doctor command should execute");

    assert!(output.status.success());
    let graph: AtlasGraph =
        serde_json::from_slice(&output.stdout).expect("doctor json should parse");
    assert_eq!(graph.schema_version, 1);
}

#[test]
fn test_doctor_with_parent_relative_repo_root_is_resolved_from_cwd() {
    let temp_dir = setup_temp_fixture("doctor-drift");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args(["doctor", "--repo-root", "..", "--format", "json"])
        .output()
        .expect("doctor command should execute");

    assert!(output.status.success());
    let graph: AtlasGraph =
        serde_json::from_slice(&output.stdout).expect("doctor json should parse");
    assert_eq!(graph.schema_version, 1);
}

// ============================================================================
// QUERY COMMAND TESTS
// ============================================================================

#[test]
fn test_query_by_id() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "req:example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("req:example"))
        .stdout(predicate::str::contains("requirement"))
        .stdout(predicate::str::contains("Example requirement"));
}

#[test]
fn test_query_by_title() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("scen:example-build"))
        .stdout(predicate::str::contains("Example build"));
}

#[test]
fn test_query_by_kind() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--kind",
            "scenario",
            "example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("scen:example-build"));
}

#[test]
fn test_query_no_matches() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "nonexistent",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("no matches"));
}

#[test]
fn test_query_invalid_kind() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--kind",
            "invalid_kind",
            "example",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown node kind"));
}

#[test]
fn test_query_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    std::fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "query",
            "--repo-root",
            "..",
            "--config",
            "atlasctl-custom.toml",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("no matches"));
}

#[test]
fn test_query_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "query",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("no matches"));
}

#[test]
fn test_query_markdown_frontmatter_repo() {
    let temp_dir = setup_temp_fixture("markdown-frontmatter");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "guide",
        ])
        .assert()
        .success();
}

// ============================================================================
// WHY COMMAND TESTS
// ============================================================================

#[test]
fn test_why_by_id() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Node: scen:example-build"))
        .stdout(predicate::str::contains("Proof chain:"));
}

#[test]
fn test_why_by_unknown_id_reports_no_match_without_tips() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "scen:unknown-example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No matching node found."))
        .stdout(predicate::str::contains("Node: ").not())
        .stdout(predicate::str::contains("Tip:").not());
}

#[test]
fn test_why_by_unknown_id_json_fails() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "scen:unknown-example",
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: No matching node found"));
}

#[test]
fn test_why_by_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/engine",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Node: crate:engine"));
}

#[test]
fn test_why_by_absolute_path_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            &absolute_path,
        ])
        .output()
        .expect("why command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Node: scen:example-build"));
    assert!(stdout.contains("crates/engine/src/lib.rs"));
    assert!(!stdout.contains(&repo_root));
    assert!(!stdout.contains("\\"));
}

#[test]
fn test_why_by_absolute_path_with_relative_repo_root_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args(["why", "--repo-root", ".", "--path", &absolute_path])
        .output()
        .expect("why command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Node: scen:example-build"));
    assert!(stdout.contains("crates/engine/src/lib.rs"));
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_why_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    std::fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "why",
            "--repo-root",
            "..",
            "--path",
            "crates/engine/src/lib.rs",
            "--config",
            "atlasctl-custom.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("why command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: No matching node found"));
}

#[test]
fn test_why_by_backslash_path_still_finds_node() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates\\engine\\src\\lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Proof chain:"));
}

#[test]
fn test_why_by_missing_path_reports_no_match() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/atlasctl-core/src/lib.r",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No matching node found."))
        .stdout(predicate::str::contains(
            "Tip: check the path spelling, or add atlas metadata coverage for this path.",
        ));
}

#[test]
fn test_why_by_missing_absolute_path_reports_no_match() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/atlasctl-core/src/lib.r")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            &absolute_path,
        ])
        .output()
        .expect("why command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No matching node found."));
    assert!(
        stdout.contains(
            "Tip: check the path spelling, or add atlas metadata coverage for this path.",
        )
    );
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_why_markdown_by_missing_absolute_path_with_relative_repo_root_reports_no_match() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/atlasctl-core/src/lib.r")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "why",
            "--repo-root",
            ".",
            "--path",
            &absolute_path,
            "--format",
            "markdown",
        ])
        .output()
        .expect("why command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: No matching node found"));
    assert!(!stderr.contains(&absolute_path));
    assert!(!stderr.contains(&repo_root));
    assert!(!stderr.contains("\\"));
}

#[test]
fn test_impacted_by_absolute_path_with_relative_repo_root_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "impacted",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);

    let changed = payload
        .payload
        .changed_paths
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();

    assert_eq!(changed, vec!["crates/engine/src/lib.rs".to_string()]);
}

#[test]
fn test_impacted_by_absolute_path_with_parent_relative_repo_root_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "impacted",
            "--repo-root",
            "..",
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);

    let changed = payload
        .payload
        .changed_paths
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();

    assert_eq!(changed, vec!["crates/engine/src/lib.rs".to_string()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
}

#[test]
fn test_impacted_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    std::fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "impacted",
            "--repo-root",
            "..",
            "--paths",
            "crates/engine/src/lib.rs",
            "--config",
            "atlasctl-custom.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);
    let changed_paths = payload
        .payload
        .changed_paths
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();
    let uncovered_paths = payload
        .payload
        .uncovered
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();

    assert_eq!(changed_paths, vec!["crates/engine/src/lib.rs".to_string()]);
    assert_eq!(
        uncovered_paths,
        vec!["crates/engine/src/lib.rs".to_string()]
    );
}

#[test]
fn test_impacted_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    std::fs::write(&config_path, "[discovery]\nroots = []\n").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "impacted",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);
    let changed_paths = payload
        .payload
        .changed_paths
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();
    let uncovered_paths = payload
        .payload
        .uncovered
        .into_iter()
        .map(|path| path.path.as_str().to_string())
        .collect::<Vec<_>>();

    assert_eq!(changed_paths, vec!["crates/engine/src/lib.rs".to_string()]);
    assert_eq!(
        uncovered_paths,
        vec!["crates/engine/src/lib.rs".to_string()]
    );
}

#[test]
fn test_why_by_missing_windows_absolute_path_reports_no_match() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let windows_style_path = "C:\\not\\a\\real\\path.rs".to_string();
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            &windows_style_path,
        ])
        .output()
        .expect("why command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No matching node found."));
    assert!(
        stdout.contains(
            "Tip: check the path spelling, or add atlas metadata coverage for this path.",
        )
    );
    assert!(!stdout.contains(&windows_style_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_why_by_existing_path_without_metadata_suggests_coverage_tip() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let orphan_path = temp_dir.path().join("uncovered-path.txt");
    std::fs::write(&orphan_path, "uncovered input").expect("create file for orphan path test");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "uncovered-path.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No matching node found."))
        .stdout(predicate::str::contains(
            "Tip: add an `owns`/`touches` selector for this path in matching atlas metadata.",
        ));
}

#[test]
fn test_why_by_deleted_path_still_finds_owned_node() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let deleted_path = temp_dir.path().join("crates/engine/src/lib.rs");
    std::fs::remove_file(&deleted_path).expect("failed to remove fixture source file");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Node: scen:example-build"))
        .stdout(predicate::str::contains("Proof chain:"));
}

#[test]
fn test_why_by_missing_path_json_fails() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "totally/missing/path.rs",
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: No matching node found"));
}

#[test]
fn test_why_by_missing_absolute_path_json_fails() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/atlasctl-core/src/lib.r")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("why command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: No matching node found"));
    assert!(!stderr.contains(&absolute_path));
    assert!(!stderr.contains(&repo_root));
}

#[test]
fn test_why_by_missing_windows_absolute_path_json_fails() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let windows_style_path = "C:\\not\\a\\real\\path.rso";
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            windows_style_path,
            "--format",
            "json",
        ])
        .output()
        .expect("why command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: No matching node found"));
    assert!(!stderr.contains(windows_style_path));
    assert!(!stderr.contains(&repo_root));
}

#[test]
fn test_why_by_path_json_returns_protocol_payload() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/engine",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("why json should parse");
    assert_eq!(value["schema_version"].as_i64(), Some(1));
    assert_eq!(value["command"].as_str(), Some("why"));
    assert_eq!(
        value["payload"]["root"]["id"].as_str(),
        Some("crate:engine")
    );
}

#[test]
fn test_why_json_paths_are_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/engine",
            "--format",
            "json",
        ])
        .output()
        .expect("why json should execute");

    assert!(output.status.success());
    let payload: WhyEnvelope =
        serde_json::from_slice(&output.stdout).expect("why json should parse");
    assert_eq!(payload.command, "why");

    let response = payload.payload;
    assert_repo_relative_source(
        response.root.provenance.source.as_str(),
        "why root provenance source",
    );

    for step in response.chain {
        assert_repo_relative_source(
            step.node.provenance.source.as_str(),
            "why chain node provenance source",
        );
    }
}

#[test]
fn test_why_json_paths_normalized_for_absolute_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("why json should execute");

    assert!(output.status.success());
    let payload: WhyEnvelope =
        serde_json::from_slice(&output.stdout).expect("why json should parse");
    assert_eq!(payload.command, "why");

    let response = payload.payload;
    assert_repo_relative_source(
        response.root.provenance.source.as_str(),
        "why root provenance source",
    );

    for step in response.chain {
        assert_repo_relative_source(
            step.node.provenance.source.as_str(),
            "why chain node provenance source",
        );
    }
}

#[test]
fn test_why_json_paths_are_normalized_for_absolute_path_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "why",
            "--repo-root",
            ".",
            "--path",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("why command should execute");

    assert!(output.status.success());
    let payload: WhyEnvelope =
        serde_json::from_slice(&output.stdout).expect("why json should parse");
    assert_eq!(payload.command, "why");
    assert_eq!(payload.schema_version, 1);

    let response = payload.payload;
    assert_repo_relative_source(
        response.root.provenance.source.as_str(),
        "why root provenance source",
    );

    for step in response.chain {
        assert_repo_relative_source(
            step.node.provenance.source.as_str(),
            "why chain node provenance source",
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
    assert!(!stdout.contains("\\"));
}

#[test]
fn test_why_missing_absolute_path_json_fails_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/atlasctl-core/src/lib.r")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "why",
            "--repo-root",
            ".",
            "--path",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("why command should execute");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: No matching node found"));
    assert!(!stderr.contains(&absolute_path));
    assert!(!stderr.contains(&repo_root));
    assert!(!stderr.contains("\\"));
}

#[test]
fn test_why_by_missing_path_review_packet_fails() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "totally/missing/path.rs",
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: No matching node found"));
}

#[test]
fn test_why_by_glob_path_matches_recursive_touch_selector() {
    let temp_dir = setup_temp_fixture("why-glob");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "crates/**/*.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Node: scen:glob-touch"));
}

#[test]
fn test_why_by_policy_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_config = r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs", "policy"]
ignore = ["target", ".git"]
"#;
    std::fs::write(temp_dir.path().join("atlas.toml"), atlas_config).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("policy")).unwrap();
    let policy_file = r#"
[atlas]
id = "policy_ledger:cli-policy"
kind = "policy_ledger"
title = "CLI policy test"
summary = "Policy ledger for CLI integration tests."
surfaces = ["policy/**/*.toml"]
proves = ["cmd:docs-check"]
"#;
    std::fs::write(temp_dir.path().join("policy/cli-policy.toml"), policy_file).unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--path",
            "policy/cli-policy.toml",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Node: policy_ledger:cli-policy"));
}

#[test]
fn test_why_markdown_format() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "markdown",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- **Schema version**: `1`"))
        .stdout(predicate::str::contains("# Why: `scen:example-build`"));
}

#[test]
fn test_why_gh_summary_format() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "gh-summary",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- **Schema version**: `1`"))
        .stdout(predicate::str::contains("# Why: `scen:example-build`"));
}

// ============================================================================
// TRACE COMMAND TESTS
// ============================================================================

#[test]
fn test_trace_outgoing() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--direction",
            "outgoing",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: scen:example-build"))
        .stdout(predicate::str::contains("nodes:"))
        .stdout(predicate::str::contains("edges:"));
}

#[test]
fn test_trace_incoming() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--direction",
            "incoming",
            "req:example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: req:example"))
        .stdout(predicate::str::contains("scen:example-build"));
}

#[test]
fn test_trace_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "trace",
            "--repo-root",
            "..",
            "--config",
            "atlasctl-custom.toml",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: scen:example-build"));
}

#[test]
fn test_trace_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "trace",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: scen:example-build"));
}

#[test]
fn test_trace_both_directions() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--direction",
            "both",
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: scen:example-build"));
}

#[test]
fn test_trace_with_max_depth() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--max-depth",
            "1",
            "scen:example-build",
        ])
        .assert()
        .success();
}

#[test]
fn test_trace_nonexistent_node() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "nonexistent:node",
        ])
        .assert()
        .code(3) // Exit code 3 for validation failure when root not found
        .stdout(predicate::str::contains("trace root not found"));
}

#[test]
fn test_trace_invalid_node_id() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "invalid-id-format",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trace root"));
}

// ============================================================================
// IMPACTED COMMAND TESTS
// ============================================================================

#[test]
fn test_impacted_by_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Impact Analysis:"))
        .stdout(predicate::str::contains("crate:engine"))
        .stdout(predicate::str::contains("scen:example-build"));
}

#[test]
fn test_impacted_by_directory_path_expands_nested_files() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_config = r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs", "policy"]
ignore = ["target", ".git"]
"#;
    std::fs::write(temp_dir.path().join("atlas.toml"), atlas_config).unwrap();
    let policy_dir = temp_dir.path().join("policy");
    std::fs::create_dir_all(&policy_dir).unwrap();
    std::fs::create_dir_all(policy_dir.join("nested")).unwrap();
    let policy_file = r#"
 [atlas]
id = "policy_ledger:cli-policy"
kind = "policy_ledger"
title = "CLI policy test"
summary = "Policy ledger for CLI integration tests."
surfaces = ["policy/**/*.toml"]
proves = ["cmd:docs-check"]
"#;
    std::fs::write(policy_dir.join("cli-policy.toml"), policy_file).unwrap();
    std::fs::write(policy_dir.join("other.txt"), b"supporting file".as_slice()).unwrap();
    std::fs::write(
        policy_dir.join("nested").join("nested-policy.toml"),
        policy_file,
    )
    .unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "policy/",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run atlasctl-cli");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: Value = serde_json::from_str(&stdout).unwrap();
    let payload = payload["payload"].as_object().unwrap();
    let changed_paths = payload["changed_paths"].as_array().unwrap();
    let covered_paths: Vec<_> = changed_paths
        .iter()
        .map(|p| p["path"].as_str().unwrap())
        .collect();

    assert!(covered_paths.contains(&"policy/cli-policy.toml"));
    assert!(covered_paths.contains(&"policy/nested/nested-policy.toml"));
    assert!(!payload["uncovered"].as_array().unwrap().is_empty());
    assert!(
        payload["uncovered"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path["path"] == "policy/other.txt")
    );

    let impacted_ids: Vec<_> = payload["impacted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["node"]["id"].as_str().unwrap().to_string())
        .collect();
    assert!(impacted_ids.contains(&"policy_ledger:cli-policy".to_string()));
}

#[test]
fn test_impacted_json_paths_are_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("impacted json should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);

    for changed_path in &payload.payload.changed_paths {
        assert_repo_relative_path(changed_path.path.as_str(), "impact changed path");
    }
    for uncovered in &payload.payload.uncovered {
        assert_repo_relative_path(uncovered.path.as_str(), "impact uncovered path");
    }

    for diagnostic in &payload.payload.missing_evidence {
        if let Some(location) = &diagnostic.location {
            assert_repo_relative_path(location.path.as_str(), "missing evidence location");
        }
    }
}

#[test]
fn test_impacted_json_paths_are_repo_relative_for_absolute_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted json should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");

    for changed_path in &payload.payload.changed_paths {
        assert_repo_relative_path(changed_path.path.as_str(), "impact changed path");
    }
    for uncovered in &payload.payload.uncovered {
        assert_repo_relative_path(uncovered.path.as_str(), "impact uncovered path");
    }
}

#[test]
fn test_impacted_by_multiple_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
            "atlas/example.atlas.yaml",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Impact Analysis:"));
}

#[test]
fn test_impacted_uncovered() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "unknown/file.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("uncovered changes: 1"))
        .stdout(predicate::str::contains("status: warnings"))
        .stdout(predicate::str::contains("- unknown/file.txt"));
}

#[test]
fn test_impacted_uncovered_ci_warning() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "ci",
            "--paths",
            "unknown/file.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: warnings"))
        .stdout(predicate::str::contains("- unknown/file.txt"));
}

#[test]
fn test_impacted_uncovered_strict_error() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "strict",
            "--paths",
            "unknown/file.txt",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("status: errors"));
}

#[test]
fn test_impacted_by_absolute_path_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: ImpactEnvelope = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);
    let changed = payload
        .payload
        .changed_paths
        .first()
        .map(|path| path.path.as_str())
        .unwrap_or("");
    assert_eq!(changed, "crates/engine/src/lib.rs");
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_impacted_by_missing_absolute_path_is_not_absolute() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");

    for changed_path in &payload.payload.changed_paths {
        assert_repo_relative_path(changed_path.path.as_str(), "impact missing changed path");
        assert_eq!(changed_path.path.as_str(), "not/a/real/path.rs");
    }

    for uncovered in &payload.payload.uncovered {
        assert_repo_relative_path(uncovered.path.as_str(), "impact missing uncovered path");
        assert_eq!(uncovered.path.as_str(), "not/a/real/path.rs");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let normalized = "not/a/real/path.rs";
    assert!(stdout.contains(normalized));
    assert!(!stdout.contains(&absolute_path));
}

#[test]
fn test_impacted_by_missing_absolute_path_is_not_absolute_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "impacted",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");

    for changed_path in &payload.payload.changed_paths {
        assert_repo_relative_path(changed_path.path.as_str(), "impact missing changed path");
        assert_eq!(changed_path.path.as_str(), "not/a/real/path.rs");
    }

    for uncovered in &payload.payload.uncovered {
        assert_repo_relative_path(uncovered.path.as_str(), "impact missing uncovered path");
        assert_eq!(uncovered.path.as_str(), "not/a/real/path.rs");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_impacted_markdown_by_missing_absolute_path_is_not_absolute() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not/a/real/path.rs"));
    assert!(!stdout.contains(&absolute_path));
}

#[test]
fn test_impacted_markdown_by_missing_absolute_path_is_not_absolute_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "impacted",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "markdown",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not/a/real/path.rs"));
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_impacted_json_dedups_and_sorts_paths_from_mixed_inputs() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let zeta_path = temp_dir.path().join("crates").join("zeta.txt");
    let alpha_path = temp_dir.path().join("crates").join("alpha.txt");
    let beta_path = temp_dir.path().join("crates").join("beta.txt");
    fs::write(&zeta_path, "zeta").unwrap();
    fs::write(&alpha_path, "alpha").unwrap();
    fs::write(&beta_path, "beta").unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            zeta_path.to_str().unwrap(),
            alpha_path.to_str().unwrap(),
            "crates/beta.txt",
            "crates/alpha.txt",
            zeta_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");

    let changed: Vec<_> = payload
        .payload
        .changed_paths
        .iter()
        .map(|path| path.path.as_str())
        .collect();
    let uncovered: Vec<_> = payload
        .payload
        .uncovered
        .iter()
        .map(|path| path.path.as_str())
        .collect();

    let expected = vec!["crates/alpha.txt", "crates/beta.txt", "crates/zeta.txt"];
    assert_eq!(changed, expected);
    assert_eq!(uncovered, expected);
    assert!(payload.payload.uncovered.iter().all(|path| {
        let value = path.path.as_str();
        assert_repo_relative_path(value, "impact uncovered path");
        !value.is_empty()
    }));
}

#[test]
fn test_impacted_by_missing_windows_absolute_path_is_not_absolute() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let windows_style_path = "C:\\not\\a\\real\\path.rs".to_string();
    let expected_path = "not/a/real/path.rs";
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &windows_style_path,
            "--format",
            "json",
        ])
        .output()
        .expect("impacted command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("impacted json should parse");
    assert_eq!(payload.command, "impacted");
    assert_eq!(payload.schema_version, 1);

    for changed_path in &payload.payload.changed_paths {
        assert_repo_relative_path(changed_path.path.as_str(), "impact windows changed path");
        assert_eq!(changed_path.path.as_str(), expected_path);
    }

    for uncovered in &payload.payload.uncovered {
        assert_repo_relative_path(uncovered.path.as_str(), "impact windows uncovered path");
        assert_eq!(uncovered.path.as_str(), expected_path);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(expected_path));
    assert!(!stdout.contains(&windows_style_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_impacted_backslash_path_is_normalized() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates\\engine\\src\\lib.rs",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 📈 Impact Summary"))
        .stdout(predicate::str::contains("- Changed paths: `1`"));
}

#[test]
fn test_review_packet_command_json() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates\\engine\\src\\lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout).expect("review-packet json should parse");

    assert_eq!(value["command"], "review-packet");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(
        value["payload"]["changed_paths"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        value["payload"]["changed_paths"][0]["path"],
        Value::String("crates/engine/src/lib.rs".to_string())
    );
}

#[test]
fn test_review_packet_json_paths_are_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet json should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    for path in &payload.payload.changed_paths {
        assert_repo_relative_path(path.path.as_str(), "impacted changed path");
    }
    for path in &payload.payload.uncovered {
        assert_repo_relative_path(path.path.as_str(), "impacted uncovered path");
    }
}

#[test]
fn test_review_packet_json_includes_uncovered_next_actions() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "uu/file-a.txt",
            "uu/file-b.txt",
            "uu/file-c.txt",
            "uu/file-d.txt",
            "uu/file-e.txt",
            "uu/file-f.txt",
            "uu/file-g.txt",
            "uu/file-h.txt",
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet json should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout).expect("review-packet json should parse");

    let suggested_fixes = value["payload"]["suggested_fixes"]
        .as_array()
        .expect("suggested_fixes should be an array");
    assert!(
        suggested_fixes
            .iter()
            .any(|fix| fix.as_str().is_some_and(|s| s.contains("uu/file-a.txt")))
    );
    assert!(suggested_fixes.iter().any(|fix| {
        fix.as_str()
            .is_some_and(|s| s.contains("3 additional uncovered paths"))
    }));
}

#[test]
fn test_review_packet_path_normalization() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "markdown",
            "--paths",
            "crates\\engine\\src\\lib.rs",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("`crates/engine/src/lib.rs`"));
    assert!(!stdout.contains("crates\\engine\\src\\lib.rs"));
}

#[test]
fn test_review_packet_path_normalization_for_absolute_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "markdown",
            "--paths",
            &absolute_path,
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Schema version: `1`"));
    assert!(stdout.contains("`crates/engine/src/lib.rs`"));
    assert!(!stdout.contains("`crates\\engine\\src\\lib.rs`"));
    assert!(!stdout.contains(&format!("`{absolute_path}`")));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_path_normalization_for_absolute_paths_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "review-packet",
            "--repo-root",
            ".",
            "--format",
            "markdown",
            "--paths",
            &absolute_path,
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Schema version: `1`"));
    assert!(stdout.contains("`crates/engine/src/lib.rs`"));
    assert!(!stdout.contains("`crates\\engine\\src\\lib.rs`"));
    assert!(!stdout.contains(&format!("`{absolute_path}`")));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_json_paths_are_repo_relative_for_absolute_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    for path in &payload.payload.changed_paths {
        assert_repo_relative_path(path.path.as_str(), "impact changed path");
    }
    for path in &payload.payload.uncovered {
        assert_repo_relative_path(path.path.as_str(), "impact uncovered path");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_json_paths_are_repo_relative_for_absolute_path_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "review-packet",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    for path in &payload.payload.changed_paths {
        assert_repo_relative_path(path.path.as_str(), "impact changed path");
    }

    assert!(!output.stdout.iter().any(|b| b == &b'\\'));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_json_paths_for_missing_absolute_path_are_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    for path in &payload.payload.changed_paths {
        assert_repo_relative_path(path.path.as_str(), "impact changed path");
        assert_eq!(path.path.as_str(), "not/a/real/path.rs");
    }

    for path in &payload.payload.uncovered {
        assert_repo_relative_path(path.path.as_str(), "impact uncovered path");
        assert_eq!(path.path.as_str(), "not/a/real/path.rs");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
}

#[test]
fn test_review_packet_json_paths_for_missing_absolute_path_are_repo_relative_with_relative_repo_root()
 {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "review-packet",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    for path in &payload.payload.changed_paths {
        assert_repo_relative_path(path.path.as_str(), "impact changed path");
        assert_eq!(path.path.as_str(), "not/a/real/path.rs");
    }

    for path in &payload.payload.uncovered {
        assert_repo_relative_path(path.path.as_str(), "impact uncovered path");
        assert_eq!(path.path.as_str(), "not/a/real/path.rs");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_json_suggested_fixes_normalize_missing_path_input() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "json",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review-packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);

    let suggestions = payload.payload.suggested_fixes;
    assert!(!suggestions.is_empty());
    for suggestion in &suggestions {
        assert!(
            !suggestion.contains(&absolute_path),
            "suggested fix leaked absolute input path: {suggestion}"
        );
        assert!(
            !suggestion.contains('\\'),
            "suggested fix used backslash separator: {suggestion}"
        );
        assert!(
            !suggestion.contains("C:\\"),
            "suggested fix used windows drive format: {suggestion}"
        );
        assert!(
            !suggestion.starts_with('/'),
            "suggested fix used absolute unix path: {suggestion}"
        );
        assert!(
            !suggestion.starts_with(".."),
            "suggested fix used parent-relative path: {suggestion}"
        );
    }

    let not_a_path = "not/a/real/path.rs";
    assert!(
        suggestions
            .iter()
            .any(|suggestion| suggestion.contains(not_a_path)),
        "suggested fix should include normalized path"
    );
}

#[test]
fn test_review_packet_markdown_missing_absolute_path_is_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &absolute_path,
            "--format",
            "markdown",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not/a/real/path.rs"));
    assert!(!stdout.contains(&absolute_path));
}

#[test]
fn test_review_packet_markdown_missing_absolute_path_is_repo_relative_with_relative_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let absolute_path = temp_dir
        .path()
        .join("not/a/real/path.rs")
        .to_str()
        .unwrap()
        .to_string();
    let repo_root = temp_dir.path().to_string_lossy().replace('\\', "/");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(temp_dir.path())
        .args([
            "review-packet",
            "--repo-root",
            ".",
            "--paths",
            &absolute_path,
            "--format",
            "markdown",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not/a/real/path.rs"));
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains(&repo_root));
}

#[test]
fn test_review_packet_markdown_missing_windows_absolute_path_is_repo_relative() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let windows_style_path = "C:\\not\\a\\real\\path.rs".to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            &windows_style_path,
            "--format",
            "markdown",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not/a/real/path.rs"));
    assert!(!stdout.contains(&windows_style_path));
}

#[test]
fn test_impacted_review_packet() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# 📦 Atlas Review Packet"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"))
        .stdout(predicate::str::contains("## ✅ Next Actions"));
}

#[test]
fn test_review_packet_explains_requirements_and_scenarios() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 🎯 Why this matters"))
        .stdout(predicate::str::contains(
            "- Behavioral requirements: `req:example`",
        ))
        .stdout(predicate::str::contains(
            "- Behavioral scenarios: `scen:example-build`",
        ));
}

#[test]
fn test_review_packet_includes_summary_counts() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "unknown/file.txt",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 📈 Impact Summary"))
        .stdout(predicate::str::contains("- Changed paths: `1`"))
        .stdout(predicate::str::contains("- Uncovered paths: `1`"))
        .stdout(predicate::str::contains("- Impacted nodes: `0`"));
}

#[test]
fn test_impacted_review_packet_with_base_head() {
    let (temp_dir, base, head) = setup_temp_git_fixture();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--base",
            base.as_str(),
            "--head",
            head.as_str(),
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- Schema version: `1`"))
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("## 🧪 Proof Commands to Run"))
        .stdout(predicate::str::contains("## ✅ Next Actions"))
        .stdout(predicate::str::contains("crates/engine/src/lib.rs"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"));
}

#[test]
fn test_impacted_review_packet_includes_policy_ledger_from_policy_path() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_config = r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs", "policy"]
ignore = ["target", ".git"]
"#;
    std::fs::write(temp_dir.path().join("atlas.toml"), atlas_config).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("policy")).unwrap();
    let policy_file = r#"
 [atlas]
id = "policy_ledger:cli-policy"
kind = "policy_ledger"
title = "CLI policy test"
summary = "Policy ledger for CLI integration tests."
surfaces = ["policy/**/*.toml"]
proves = ["cmd:docs-check"]
"#;
    std::fs::write(temp_dir.path().join("policy/cli-policy.toml"), policy_file).unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "policy/cli-policy.toml",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# 📦 Atlas Review Packet"))
        .stdout(predicate::str::contains("policy_ledger:cli-policy"));
}

#[test]
fn test_review_packet_with_base_head() {
    let (temp_dir, base, head) = setup_temp_git_fixture();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--base",
            base.as_str(),
            "--head",
            head.as_str(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- Schema version: `1`"))
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("## 🧪 Proof Commands to Run"))
        .stdout(predicate::str::contains("## ✅ Next Actions"))
        .stdout(predicate::str::contains("crates/engine/src/lib.rs"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"));
}

#[test]
fn test_review_packet_with_base_head_and_relative_config_path_from_nested_dir() {
    let (temp_dir, base, head) = setup_temp_git_fixture();
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "review-packet",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "--base",
            base.as_str(),
            "--head",
            head.as_str(),
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- Schema version: `1`"))
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("## 🧪 Proof Commands to Run"))
        .stdout(predicate::str::contains("## ✅ Next Actions"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"));
}

#[test]
fn test_north_star_explanation_workflow_for_git_head() {
    let (temp_dir, base, head) = setup_temp_git_fixture();
    let repo_root = temp_dir.path().to_str().unwrap();
    let absolute_path = temp_dir
        .path()
        .join("crates/engine/src/lib.rs")
        .to_str()
        .unwrap()
        .to_string();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "doctor",
            "--repo-root",
            repo_root,
            "--profile",
            "ci",
            "--format",
            "json",
        ])
        .output()
        .expect("doctor command should execute");
    assert!(output.status.success());
    let graph: AtlasGraph =
        serde_json::from_slice(&output.stdout).expect("doctor json should parse");
    assert_eq!(graph.schema_version, 1);

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            repo_root,
            "--base",
            base.as_str(),
            "--head",
            head.as_str(),
            "--format",
            "review-packet",
        ])
        .output()
        .expect("impacted command should execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- Schema version: `1`"));
    assert!(stdout.contains("## 👤 Owners"));
    assert!(stdout.contains("## 🧪 Proof Commands to Run"));
    assert!(stdout.contains("Missing Evidence"));
    assert!(stdout.contains("Scope Warnings"));
    assert!(stdout.contains("## ✅ Next Actions"));
    assert!(stdout.contains("crates/engine/src/lib.rs"));
    assert!(!stdout.contains("C:\\"));

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "why",
            "--repo-root",
            repo_root,
            "--path",
            absolute_path.as_str(),
        ])
        .output()
        .expect("why command should execute");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Node: scen:example-build"));
    assert!(stdout.contains("crates/engine/src/lib.rs"));
    assert!(!stdout.contains(&absolute_path));
    assert!(!stdout.contains("\\"));
}

#[test]
fn test_review_packet_command() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# 📦 Atlas Review Packet"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"))
        .stdout(predicate::str::contains("## ✅ Next Actions"));
}

#[test]
fn test_review_packet_multiple_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
            "atlas/example.atlas.yaml",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# 📦 Atlas Review Packet"))
        .stdout(predicate::str::contains("## 🧭 Impacted Truth Surface"))
        .stdout(predicate::str::contains("## ✅ Next Actions"));
}

// ============================================================================
// EXPORT COMMAND TESTS
// ============================================================================

#[test]
fn test_export_json_to_stdout() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"repo\""))
        .stdout(predicate::str::contains("\"nodes\""))
        .stdout(predicate::str::contains("\"edges\""));
}

#[test]
fn test_export_markdown_to_stdout() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Atlas"));
}

#[test]
fn test_export_json_to_file() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_file = temp_dir.path().join("output.json");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--out",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"repo\""));
    assert!(content.contains("\"nodes\""));
}

#[test]
fn test_export_with_parent_relative_repo_root_resolves_relative_config_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let output_file = temp_dir.path().join("custom-output.json");
    std::fs::create_dir(&nested_dir).unwrap();

    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "export",
            "--repo-root",
            "..",
            "--config",
            "atlasctl-custom.toml",
            "--format",
            "json",
            "--out",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"schema_version\""));
}

#[test]
fn test_export_with_absolute_repo_root_and_relative_config_path_from_repo_root() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    let repo_root = temp_dir
        .path()
        .canonicalize()
        .expect("fixture path should canonicalize");
    std::fs::create_dir(&nested_dir).unwrap();

    let output_file = temp_dir.path().join("custom-output-absolute-root.json");
    let config_path = temp_dir.path().join("atlasctl-custom.toml");
    fs::write(
        &config_path,
        r#"
schema_version = 1

[discovery]
roots = ["atlas", "docs"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "export",
            "--repo-root",
            repo_root.to_str().expect("repo root should be valid utf-8"),
            "--config",
            "atlasctl-custom.toml",
            "--format",
            "json",
            "--out",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"schema_version\""));
}

#[test]
fn test_export_markdown_to_file() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_file = temp_dir.path().join("output.md");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "markdown",
            "--out",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("# Atlas"));
}

#[test]
fn test_export_to_nested_directory() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_file = temp_dir.path().join("nested/dir/output.json");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--out",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_file.exists());
}

#[test]
fn test_export_with_profile() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
            "--profile",
            "ci",
        ])
        .assert()
        .success();
}

#[test]
fn test_export_broken_link_repo() {
    let temp_dir = setup_temp_fixture("broken-link");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "export",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("\"diagnostics\""));
}

// ============================================================================
// GITHUB SUMMARY TESTS
// ============================================================================

#[test]
fn test_check_gh_summary() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "check",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "gh-summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("### 🗺️ Atlas Summary"))
        .stdout(predicate::str::contains("- **Schema version**: `1`"))
        .stdout(predicate::str::contains("✅ Healthy"));
}

#[test]
fn test_impacted_gh_summary() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
            "--format",
            "gh-summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("### 🎯 Atlas Impact Analysis"))
        .stdout(predicate::str::contains("- **Schema version**: `1`"))
        .stdout(predicate::str::contains("Impacted Proof Surface"));
}

// ============================================================================
// INIT AND SCAFFOLD TESTS
// ============================================================================

#[test]
fn test_init_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["init", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized atlas"));

    let config_path = temp_dir.path().join("atlas.toml");
    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[repo]"));

    // Round-trip check: can we run check on this initialized repo?
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: ok"));
}

#[test]
fn test_init_with_parent_relative_repo_root_is_resolved_from_cwd() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();
    let expected_repo_name = temp_dir
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args(["init", "--repo-root", ".."])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized atlas"));

    let config_path = temp_dir.path().join("atlas.toml");
    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[repo]"));
    assert!(content.contains(&format!("name = \"{expected_repo_name}\"")));
}

#[test]
fn test_scaffold_scenario() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "scenario",
            "new-feature",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded scenario"));

    let scaffold_file = temp_dir.path().join("atlas/new-feature.atlas.yaml");
    assert!(scaffold_file.exists());

    // Round-trip: does the scaffolded YAML parse?
    // We don't assert .success() because the scaffold has TODO placeholders
    // which cause validation errors, but we want to see that it's discovered.
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("nodes: 7")); // 6 original + 1 scaffolded
}

#[test]
fn test_scaffold_with_parent_relative_repo_root_is_resolved_from_cwd() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let nested_dir = temp_dir.path().join("nested");
    std::fs::create_dir(&nested_dir).unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .current_dir(&nested_dir)
        .args([
            "scaffold",
            "--repo-root",
            "..",
            "scenario",
            "nested-feature",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded scenario"));

    let scaffold_file = temp_dir.path().join("atlas/nested-feature.atlas.yaml");
    assert!(scaffold_file.exists());
}

#[test]
fn test_scaffold_artifact() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "artifact",
            "new-artifact",
        ])
        .assert()
        .success();

    let scaffold_file = temp_dir.path().join("atlas/new-artifact.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("id: artifact:new-artifact"));
}

#[test]
fn test_scaffold_plan_item() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "plan-item",
            "release-plan",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded plan item"));

    let scaffold_file = temp_dir.path().join("atlas/release-plan.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: plan"));
}

#[test]
fn test_scaffold_gap() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "gap",
            "requirement_not_proven",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded gap scaffold"));

    let scaffold_file = temp_dir
        .path()
        .join("atlas/gap-requirement_not_proven.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: scenario"));
    assert!(content.contains("proves"));
    assert!(content.contains("req:todo"));
}

#[test]
fn test_scaffold_gap_normalizes_diagnostic_slug() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "gap",
            "Claim/Missing Proof!!! Command",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded gap scaffold"));

    let scaffold_file = temp_dir
        .path()
        .join("atlas/gap-claim-missing-proof-command.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("id: scen:gap-claim-missing-proof-command"));
    assert!(content.contains("kind: scenario"));
    assert!(content.contains("to: req:todo"));
}

#[test]
fn test_scaffold_support_tier() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "support-tier",
            "release-support",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded support-tier"));

    let scaffold_file = temp_dir.path().join("atlas/release-support.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: support_tier"));
    assert!(content.contains("summary: |"));
}

#[test]
fn test_scaffold_policy_ledger() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "policy-ledger",
            "release-review",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded policy-ledger"));

    let scaffold_file = temp_dir.path().join("atlas/release-review.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: policy_ledger"));
    assert!(content.contains("summary: |"));
}

#[test]
fn test_scaffold_closeout() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "closeout",
            "release-closeout",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded closeout"));

    let scaffold_file = temp_dir.path().join("atlas/release-closeout.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: closeout"));
    assert!(content.contains("id: closeout:release-closeout"));
}

#[test]
fn test_scaffold_gap_claim_missing_proof_command() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "gap",
            "claim_missing_proof_command",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded gap scaffold"));

    let scaffold_file = temp_dir
        .path()
        .join("atlas/gap-claim_missing_proof_command.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: support_tier"));
    assert!(content.contains("to: cmd:todo"));
}

#[test]
fn test_scaffold_gap_policy_ledger_missing_proof_command() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "scaffold",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "gap",
            "policy_ledger_missing_proof_command",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded gap scaffold"));

    let scaffold_file = temp_dir
        .path()
        .join("atlas/gap-policy_ledger_missing_proof_command.atlas.yaml");
    assert!(scaffold_file.exists());
    let content = fs::read_to_string(scaffold_file).unwrap();
    assert!(content.contains("kind: policy_ledger"));
    assert!(content.contains("to: cmd:todo"));
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_no_command_provided() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Commands:"));
}

#[test]
fn test_invalid_subcommand() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["invalid-command"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn test_help_flag() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Compile and inspect a repo behavior/proof atlas",
        ));
}

#[test]
fn test_build_help() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("build"));
}

#[test]
fn test_check_help() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("check"));
}

#[test]
fn test_query_help() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["query", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("query"));
}

#[test]
fn test_trace_help() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["trace", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("trace"));
}

#[test]
fn test_export_help() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export"));
}

// ============================================================================
// EXIT CODE TESTS
// ============================================================================

#[test]
fn test_exit_code_success_valid_repo() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_exit_code_validation_failure() {
    let temp_dir = setup_temp_fixture("broken-link");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure
        .stdout(predicate::str::contains("status: invalid"));
}

#[test]
fn test_exit_code_runtime_error() {
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", "/nonexistent/path"])
        .assert()
        .code(4) // Exit code 4 for runtime error
        .stderr(predicate::str::contains("error"));
}

// ============================================================================
// MARKDOWN FRONTMATTER TESTS
// ============================================================================

// Note: markdown-frontmatter fixture has a broken reference (req:frontmatter doesn't exist)
// so it will fail validation. This tests that markdown frontmatter is parsed correctly
// even when there are validation errors.
#[test]
fn test_build_markdown_frontmatter_repo() {
    let temp_dir = setup_temp_fixture("markdown-frontmatter");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure (has errors)
        .stdout(predicate::str::contains("status: invalid"));
}

#[test]
fn test_check_markdown_frontmatter_repo() {
    let temp_dir = setup_temp_fixture("markdown-frontmatter");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["check", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .code(3) // Exit code 3 for validation failure (has errors)
        .stdout(predicate::str::contains("status: invalid"));
}

// ============================================================================
// COMPLEX SCENARIO TESTS
// ============================================================================

#[test]
fn test_build_then_query() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    // First build
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();

    // Then query
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "query",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("scen:example-build"));
}

#[test]
fn test_build_then_trace() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    // First build
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args(["build", "--repo-root", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();

    // Then trace
    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "trace",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "scen:example-build",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("root: scen:example-build"));
}

#[test]
fn test_multiple_formats_build() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let output_dir = temp_dir.path().join(".atlas");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "build",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--out-dir",
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify both formats were created
    assert!(
        output_dir.join("atlas.json").exists(),
        "atlas.json not found in {:?}",
        output_dir
    );
    assert!(
        output_dir.join("atlas.md").exists(),
        "atlas.md not found in {:?}",
        output_dir
    );
}

#[test]
fn test_review_packet_next_actions_include_scope_warnings() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "schemas/atlas.schema.json .github/workflows/ci.yml",
            "--format",
            "review-packet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "schema change is not linked to a protocol spec/proposal/doc artifact",
        ))
        .stdout(predicate::str::contains(
            "link the changed schema to a protocol spec or proposal in atlas metadata",
        ))
        .stdout(predicate::str::contains(
            "add or update the impacted `policy_ledger` node for workflow changes",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_alias_includes_scope_next_actions() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "schemas/atlas.schema.json .github/workflows/ci.yml",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "schema change is not linked to a protocol spec/proposal/doc artifact",
        ))
        .stdout(predicate::str::contains(
            "link the changed schema to a protocol spec or proposal in atlas metadata",
        ))
        .stdout(predicate::str::contains(
            "add or update the impacted `policy_ledger` node for workflow changes",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_includes_owners_section_from_graph_metadata() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("- scen:example-build"));
}

#[test]
fn test_review_packet_includes_ownership_by_path_section() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let codeowners = temp_dir.path().join("CODEOWNERS");
    fs::write(&codeowners, "crates/engine/src/lib.rs @engine-team\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 👥 Ownership by Path"))
        .stdout(predicate::str::contains(
            "- `crates/engine/src/lib.rs`: @engine-team",
        ));
}

#[test]
fn test_review_packet_includes_proof_reasons_in_proof_command_list() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 🧪 Proof Commands to Run"))
        .stdout(predicate::str::contains(
            "`cmd:ci-fast` — Fast CI (related to `scen:example-build` via `runs_with`)",
        ));
}

#[test]
fn test_review_packet_includes_active_goal_context() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    std::fs::create_dir_all(temp_dir.path().join(".codex/goals")).unwrap();
    std::fs::write(
        temp_dir.path().join(".codex/goals/active.toml"),
        r#"goal = "goal:ship-proof-topology-stack"
plan = "plan:post-closeout-review-surface-hardening"
proposal = "proposal:review-packet-router"
spec = "spec:router-proof-contract"
ready_work_items = ["scen:example-build"]
"#,
    )
    .unwrap();
    std::fs::write(
        temp_dir.path().join("atlas/active-goal-context.atlas.yaml"),
        r#"nodes:
  - id: goal:ship-proof-topology-stack
    kind: goal
    title: Ship proof topology stack
    touches:
      - crates/engine/src/lib.rs
  - id: plan:post-closeout-review-surface-hardening
    kind: plan
    title: Post-closeout review-surface hardening
    touches:
      - crates/engine/src/lib.rs
  - id: proposal:review-packet-router
    kind: proposal
    title: Review packet router proposal
    touches:
      - crates/engine/src/lib.rs
  - id: spec:router-proof-contract
    kind: spec
    title: Review packet governance spec
    touches:
      - crates/engine/src/lib.rs
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 🎯 Active Goal Context"))
        .stdout(predicate::str::contains(
            "- Goal: `goal:ship-proof-topology-stack` ✅ impacted",
        ))
        .stdout(predicate::str::contains(
            "- Plan: `plan:post-closeout-review-surface-hardening` ✅ impacted",
        ))
        .stdout(predicate::str::contains(
            "- Proposal: `proposal:review-packet-router` ✅ impacted",
        ))
        .stdout(predicate::str::contains(
            "- Spec: `spec:router-proof-contract` ✅ impacted",
        ))
        .stdout(predicate::str::contains("- Ready work items:"))
        .stdout(predicate::str::contains("scen:example-build` ✅ impacted"));
}

#[test]
fn test_review_packet_next_actions_include_active_goal_ready_work_item_suggestion() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    std::fs::create_dir_all(temp_dir.path().join(".codex/goals")).unwrap();
    std::fs::write(
        temp_dir.path().join(".codex/goals/active.toml"),
        r#"goal = "goal:ship-proof-topology-stack"
plan = "plan:post-closeout-review-surface-hardening"
proposal = "proposal:review-packet-router"
spec = "spec:router-proof-contract"
ready_work_items = ["scen:future-scenario", "scen:example-build"]
"#,
    )
    .unwrap();
    std::fs::write(
        temp_dir.path().join("atlas/active-goal-context.atlas.yaml"),
        r#"nodes:
  - id: goal:ship-proof-topology-stack
    kind: goal
    title: Ship proof topology stack
    touches:
      - crates/engine/src/lib.rs
  - id: plan:post-closeout-review-surface-hardening
    kind: plan
    title: Post-closeout review-surface hardening
    touches:
      - crates/engine/src/lib.rs
  - id: proposal:review-packet-router
    kind: proposal
    title: Review packet router proposal
    touches:
      - crates/engine/src/lib.rs
  - id: spec:router-proof-contract
    kind: spec
    title: Review packet governance spec
    touches:
      - crates/engine/src/lib.rs
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## ✅ Next Actions"))
        .stdout(predicate::str::contains(
            "Advance active goal work item `scen:future-scenario` in the next PR.",
        ));
}

#[test]
fn test_review_packet_next_actions_include_invalid_active_goal_work_item_warning() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    std::fs::create_dir_all(temp_dir.path().join(".codex/goals")).unwrap();
    std::fs::write(
        temp_dir.path().join(".codex/goals/active.toml"),
        r#"goal = "goal:ship-proof-topology-stack"
plan = "plan:post-closeout-review-surface-hardening"
proposal = "proposal:proof-topology-stack"
spec = "spec:router-proof-contract"
ready_work_items = ["!!!not-a-valid-id"]
"#,
    )
    .unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## ✅ Next Actions"))
        .stdout(predicate::str::contains(
            "Fix active goal work item `!!!not-a-valid-id`: invalid atlas id.",
        ));
}

#[test]
fn test_impacted_json_includes_active_goal_context() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    std::fs::create_dir_all(temp_dir.path().join(".codex/goals")).unwrap();
    std::fs::write(
        temp_dir.path().join(".codex/goals/active.toml"),
        r#"goal = "goal:ship-proof-topology-stack"
plan = "plan:post-closeout-review-surface-hardening"
proposal = "proposal:review-packet-router"
spec = "spec:router-proof-contract"
ready_work_items = ["scen:example-build"]
"#,
    )
    .unwrap();
    std::fs::write(
        temp_dir.path().join("atlas/active-goal-context.atlas.yaml"),
        r#"nodes:
  - id: goal:ship-proof-topology-stack
    kind: goal
    title: Ship proof topology stack
    touches:
      - crates/engine/src/lib.rs
  - id: plan:post-closeout-review-surface-hardening
    kind: plan
    title: Post-closeout review-surface hardening
    touches:
      - crates/engine/src/lib.rs
  - id: proposal:review-packet-router
    kind: proposal
    title: Review packet router proposal
    touches:
      - crates/engine/src/lib.rs
  - id: spec:router-proof-contract
    kind: spec
    title: Review packet governance spec
    touches:
      - crates/engine/src/lib.rs
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "impacted",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run atlasctl-cli");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let payload: Value = serde_json::from_str(&stdout).unwrap();
    let payload = &payload["payload"];
    let active = payload["active_goal"]
        .as_object()
        .expect("active goal should exist");
    assert_eq!(active["goal"], "goal:ship-proof-topology-stack");
    assert_eq!(
        active["plan"],
        "plan:post-closeout-review-surface-hardening"
    );
    assert_eq!(active["proposal"], "proposal:review-packet-router");
    assert_eq!(active["spec"], "spec:router-proof-contract");

    let ready = active["ready_work_items"]
        .as_array()
        .expect("ready work items should be array");
    assert!(ready.contains(&Value::String("scen:example-build".into())));
}

#[test]
fn test_review_packet_uses_codeowners_for_owners_section() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let codeowners = temp_dir.path().join("CODEOWNERS");
    fs::write(&codeowners, "crates/engine/src/lib.rs @engine-team\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("- @engine-team"));
}

#[test]
fn test_review_packet_alias_next_actions_include_scope_mix_warning() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs docs/adr/0001-example.md",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "path set mixes documentation and implementation files",
        ))
        .stdout(predicate::str::contains(
            "split the review scope to avoid docs/implementation mix in one change",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_does_not_mix_docs_and_generated_as_scope_mix() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "docs/adr/0001-example.md",
            "target/atlas.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "generated artifact changed without an impacted artifact node",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_next_actions_include_uncovered_paths_warning() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "unknown/file.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "changed paths are not covered by any known ownership/touches selector",
        ))
        .stdout(predicate::str::contains(
            "Add `owns`/`touches` coverage for changed path `unknown/file.txt`",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_next_actions_group_multiple_uncovered_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "uu/file-a.txt",
            "uu/file-b.txt",
            "uu/file-c.txt",
            "uu/file-d.txt",
            "uu/file-e.txt",
            "uu/file-f.txt",
            "uu/file-g.txt",
            "uu/file-h.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "changed paths are not covered by any known ownership/touches selector",
        ))
        .stdout(predicate::str::contains(
            "Add `owns`/`touches` coverage for changed path `uu/file-a.txt`",
        ))
        .stdout(predicate::str::contains(
            "Add `owns`/`touches` coverage for changed path `uu/file-e.txt`",
        ))
        .stdout(predicate::str::contains(
            "3 additional uncovered paths need explicit `owns`/`touches` coverage.",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_next_actions_include_touches_only_warning() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_yaml = r#"nodes:
  - id: scen:touch-only
    kind: scenario
    title: Touch-only scenario
    touches:
      - crates/engine/src/lib.rs

  - id: req:touch-only
    kind: requirement
    title: Touch-only requirement

  - id: cmd:touch-test
    kind: command
    title: Touch test command

  - id: crate:engine
    kind: crate
    title: Engine crate

edges:
  - from: scen:touch-only
    kind: proves
    to: req:touch-only
  - from: scen:touch-only
    kind: runs_with
    to: cmd:touch-test
  - from: scen:touch-only
    kind: exercises
    to: crate:engine
"#;
    fs::write(temp_dir.path().join("atlas/example.atlas.yaml"), atlas_yaml).unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine/src/lib.rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scope Warnings"))
        .stdout(predicate::str::contains(
            "`crates/engine/src/lib.rs` is only covered by `touches` metadata and has no explicit `owns` coverage",
        ))
        .stdout(predicate::str::contains(
            "replace `touches` metadata with explicit `owns` coverage for accountable ownership",
        ))
        .stdout(predicate::str::contains("Next Actions"));
}

#[test]
fn test_review_packet_includes_missing_evidence_for_unproven_requirement() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_yaml = r#"nodes:
  - id: req:example
    kind: requirement
    title: Example requirement

  - id: req:unproven
    kind: requirement
    title: Unproven requirement
    touches:
      - crates/engine/src/lib.rs

  - id: scen:example-build
    kind: scenario
    title: Example build
    paths:
      - crates/engine/src/lib.rs

  - id: cmd:ci-fast
    kind: command
    title: Fast CI

edges:
  - from: scen:example-build
    kind: proves
    to: req:example
  - from: scen:example-build
    kind: runs_with
    to: cmd:ci-fast
  - from: scen:example-build
    kind: exercises
    to: crate:engine
  - from: cmd:ci-fast
    kind: implements
    to: req:unproven
"#;
    fs::write(temp_dir.path().join("atlas/example.atlas.yaml"), atlas_yaml).unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "ci",
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "review-packet",
        ])
        .output()
        .expect("review-packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("## ⚠️ Missing Evidence"));
    assert!(stdout.contains("requirement_not_proven"));
    assert!(stdout.contains("Missing Evidence"));
    assert!(stdout.contains("Owners"));
    assert!(stdout.contains("req:unproven"));
    assert!(stdout.contains("## 👤 Owners"));
    assert!(stdout.contains("Scope Warnings"));
    assert!(stdout.contains("Next Actions"));
    assert!(stdout.contains("atlasctl scaffold gap requirement_not_proven"));
}

#[test]
fn test_review_packet_json_includes_missing_evidence_and_next_actions_for_unproven_requirement() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let atlas_yaml = r#"nodes:
  - id: req:example
    kind: requirement
    title: Example requirement

  - id: req:unproven
    kind: requirement
    title: Unproven requirement
    touches:
      - crates/engine/src/lib.rs

  - id: scen:example-build
    kind: scenario
    title: Example build
    paths:
      - crates/engine/src/lib.rs

  - id: cmd:ci-fast
    kind: command
    title: Fast CI

edges:
  - from: scen:example-build
    kind: proves
    to: req:example
  - from: scen:example-build
    kind: runs_with
    to: cmd:ci-fast
  - from: scen:example-build
    kind: exercises
    to: crate:engine
  - from: cmd:ci-fast
    kind: implements
    to: req:unproven
"#;
    fs::write(temp_dir.path().join("atlas/example.atlas.yaml"), atlas_yaml).unwrap();

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--profile",
            "ci",
            "--paths",
            "crates/engine/src/lib.rs",
            "--format",
            "json",
        ])
        .output()
        .expect("impact command should execute");

    assert!(output.status.success());
    let payload: ImpactEnvelope =
        serde_json::from_slice(&output.stdout).expect("review packet json should parse");
    assert_eq!(payload.command, "review-packet");
    assert_eq!(payload.schema_version, 1);
    assert!(
        payload
            .payload
            .missing_evidence
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::RequirementNotProven),
        "expected requirement_not_proven diagnostic"
    );
    assert!(
        payload
            .payload
            .suggested_fixes
            .iter()
            .any(|fix| fix
                == "add a scenario that proves this requirement and connects to a command"),
        "expected next-action suggestion for missing requirement evidence"
    );
    assert!(
        payload
            .payload
            .suggested_fixes
            .iter()
            .any(|fix| fix
                == "run `atlasctl scaffold gap requirement_not_proven` to create starter metadata for this missing proof"),
        "expected scaffold next-action suggestion for missing requirement evidence"
    );
}

#[test]
fn test_review_packet_uses_codeowners_for_uncovered_paths() {
    let temp_dir = setup_temp_fixture("valid-minimal");
    let codeowners = temp_dir.path().join("CODEOWNERS");
    fs::write(&codeowners, "unknown/file.txt @reviewer\n").unwrap();

    Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "unknown/file.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("## 👤 Owners"))
        .stdout(predicate::str::contains("- @reviewer"))
        .stdout(predicate::str::contains("Scope Warnings"));
}
#[test]
fn test_review_packet_includes_schema_version_in_markdown() {
    let temp_dir = setup_temp_fixture("valid-minimal");

    let output = Command::cargo_bin("atlasctl-cli")
        .unwrap()
        .args([
            "review-packet",
            "--repo-root",
            temp_dir.path().to_str().unwrap(),
            "--paths",
            "crates/engine",
            "--format",
            "review-packet",
        ])
        .output()
        .expect("review packet command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Schema version: `1`"),
        "review packet markdown should include protocol schema version"
    );
}
