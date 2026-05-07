# TESTING

This document describes the testing strategy and how to run tests for the atlasctl project.

## Verification stack

This repo follows a layered verification model.

### Types and codes
- unit tests
- property tests for stable parsing and ordering

### Core
- scenario tests for validation, trace semantics, why proof chains, and impact expansion
- property tests for determinism and integrity (including path normalization and absolute path rejection)
- mutation testing on diff-critical paths

### Discovery
- fixture-repo tests
- malformed fragment tests
- frontmatter parsing tests
- codeowner overlay tests

### Render
- snapshot and golden tests for `atlas.json`, `atlas.md`, and GitHub step summaries (`gh-summary.md`)

### CLI
- smoke tests
- exit-code tests
- end-to-end output tests for all commands: `build`, `check`, `doctor`, `impacted`, `why`, `query`, `trace`, `init`, and `scaffold`

## Test coverage summary

The project currently has **111+ tests** across all crates:

- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON, Markdown, and GitHub summary outputs
- **CLI integration tests**: End-to-end command validation

### Test categories

| Category | Description | Location |
|----------|-------------|----------|
| Unit tests | Small, focused tests for individual functions | All crates |
| Property tests | Generative tests for invariants | `atlasctl-core`, `atlasctl-types` |
| Golden tests | Snapshot tests for output stability | `atlasctl-core` |
| Fixture tests | Tests against real repo structures | `atlasctl-discover-fs` |
| CLI tests | Integration tests for CLI commands | `atlasctl-cli/tests/` |

## Running tests

### Run all tests

```bash
cargo test --workspace
```

### Run tests for a specific package

```bash
cargo test --package atlasctl-core
cargo test --package atlasctl-discover-fs
cargo test --package atlasctl-cli
```

### Run a specific test

```bash
cargo test --package atlasctl-core test_name
```

## Golden file management

### About golden files

Golden files are snapshot tests that ensure output stability. The project uses the `insta` crate for snapshot testing.

Golden files are located in:
- `crates/atlasctl-core/src/snapshots/` - JSON and Markdown snapshots

### Updating golden files

When output changes intentionally (e.g., new fields, formatting changes):

```bash
# Install insta CLI
cargo install cargo-insta

# Review and accept all snapshots interactively
cargo insta review

# Or use the xtask command:
cargo run -p xtask -- golden
```

### Golden file discipline

`atlas.json` is the contract. If it changes intentionally:

1. update the schema if needed (`schemas/atlas.schema.json`)
2. update golden outputs
3. update docs and examples

## CI checks

The project uses `xtask` for running CI checks.

### Quick CI check

```bash
cargo run -p xtask -- ci-fast
```

This runs:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

### Full CI check

```bash
cargo run -p xtask -- ci-full
```

This runs:
- All `ci-fast` checks
- `cargo build --release`
- Self-dogfooding verification
- Additional validation

### Mutation testing

```bash
cargo run -p xtask -- mutants
```

Runs `cargo-mutants` against `atlasctl-core` to ensure that diff-critical paths (impact analysis, trace projection, validation rules) are well-protected.

## Self-dogfooding verification

The project uses `atlasctl` to verify itself:

```bash
# Check the atlas for drift and orphan nodes
cargo run -p atlasctl-cli -- doctor

# Analyze the impact of your changes
cargo run -p atlasctl-cli -- impacted --base main

# Get the proof chain for a file
cargo run -p atlasctl-cli -- why --path crates/atlasctl-core/src/lib.rs
```

## Test fixtures

The project includes fixture repositories for testing various scenarios:

- `fixtures/repos/valid-minimal/` - A minimal valid repo
- `fixtures/repos/doctor-drift/` - Repo testing `doctor` rules (orphan nodes, dead selectors)
- `fixtures/repos/broken-link/` - Repo with broken link references
- `fixtures/repos/duplicate-id/` - Repo with duplicate node IDs
- `fixtures/repos/markdown-frontmatter/` - Repo using markdown frontmatter
- `fixtures/repos/orphan-scenario/` - Repo with orphaned scenario

These fixtures are used by `atlasctl-discover-fs` and `atlasctl-core` tests to ensure discovery and validation work correctly.
