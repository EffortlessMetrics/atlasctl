# TESTING

This document describes the testing strategy and how to run tests for the atlasctl project.

## Verification stack

This repo follows a layered verification model.

### Types and codes

- unit tests
- property tests for stable parsing and ordering

### Core

- scenario tests for validation and trace semantics
- property tests for determinism and integrity
- mutation testing later on diff-critical paths

### Discovery

- fixture-repo tests
- malformed fragment tests
- frontmatter parsing tests

### Render

- snapshot and golden tests for `atlas.json` and `atlas.md`

### CLI

- smoke tests
- exit-code tests
- query and trace output tests

## Test coverage summary

The project currently has **75 tests** across all crates:

- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON and Markdown output
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

### Run tests with output

```bash
cargo test --workspace -- --nocapture
```

### Run tests in release mode

```bash
cargo test --workspace --release
```

## Golden file management

### About golden files

Golden files are snapshot tests that ensure output stability. The project uses the `insta` crate for snapshot testing.

Golden files are located in:
- `crates/atlasctl-core/src/snapshots/` - JSON and Markdown snapshots

### Updating golden files

When output changes intentionally (e.g., new fields, formatting changes):

```bash
# Review and accept all snapshots interactively
cargo test --package atlasctl-core -- --accept

# Accept without review (use with caution)
cargo test --package atlasctl-core -- --accept-unseen
```

### Reviewing snapshot changes

When running tests, if snapshots don't match, the test will fail and `cargo insta review` can be used to review changes:

```bash
# Install insta CLI (if not already installed)
cargo install cargo-insta

# Review snapshot changes interactively
cargo insta review
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

### Smoke tests

```bash
cargo run -p xtask -- smoke
```

Runs basic smoke tests to ensure the tool works end-to-end.

### Documentation check

```bash
cargo run -p xtask -- docs-check
```

Validates documentation links and structure.

## Self-dogfooding verification

The project uses `atlasctl` to verify itself:

```bash
# Build the atlas for this repo
cargo run -p atlasctl-cli -- build

# Check validation
cargo run -p atlasctl-cli -- check --profile ci

# Verify expected counts (example)
# Expected: 31 nodes, 24 edges, 0 diagnostics
```

## Test fixtures

The project includes fixture repositories for testing various scenarios:

- `fixtures/repos/valid-minimal/` - A minimal valid repo
- `fixtures/repos/broken-link/` - Repo with broken link references
- `fixtures/repos/duplicate-id/` - Repo with duplicate node IDs
- `fixtures/repos/markdown-frontmatter/` - Repo using markdown frontmatter
- `fixtures/repos/orphan-scenario/` - Repo with orphaned scenario

These fixtures are used by `atlasctl-discover-fs` tests to ensure discovery works correctly.

## Debugging test failures

### Running a single test with output

```bash
cargo test --package atlasctl-core test_name -- --nocapture --show-output
```

### Running tests with logging

```bash
RUST_LOG=debug cargo test --workspace
```

### Viewing test output

```bash
cargo test --workspace -- --test-threads=1
```

## Continuous integration

The project is configured to run tests on CI. The CI pipeline:

1. Runs `cargo run -p xtask -- ci-full`
2. Verifies all tests pass
3. Checks code formatting
4. Runs clippy with strict warnings
5. Verifies the release build

## Expected commands

```bash
# Run all tests
cargo test --workspace

# Build the atlas
cargo run -p atlasctl-cli -- build

# Check validation
cargo run -p atlasctl-cli -- check --profile ci

# Run CI checks
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full
```
