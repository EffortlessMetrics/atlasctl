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
        .stdout(predicate::str::contains("## 🎯 Impacted Proof Surface"));
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
