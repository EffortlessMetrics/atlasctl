// Integration tests for atlasctl CLI commands
// These tests verify the CLI behavior against fixture repos

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
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
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
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
        .stdout(predicate::str::contains("✅ Healthy"));
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
