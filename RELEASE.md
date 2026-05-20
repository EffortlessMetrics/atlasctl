# RELEASE

## Version 0.1.0

Release Date: 2026-04-16

### Overview

This is the v0.1.0 release of `atlasctl`, a local-first proof-topology control surface.
It compiles explicit metadata into deterministic artifacts and answers review questions around
impact, ownership, and proof coverage.

### Features Implemented

#### Core Functionality

- **Graph Model**: Stable graph model with nodes and edges representing requirements, ADRs, scenarios, fixtures, commands, artifacts, and workspace crates.
- **Deterministic Compilation**: Reproducible atlas generation from repository metadata with strict path normalization (forward slashes, repo-relative paths only).
- **Fragment Discovery**: YAML fragment discovery from the `atlas/` directory.
- **Frontmatter Parsing**: Markdown frontmatter parsing for embedded metadata.
- **Workspace Crate Discovery**: Automatic discovery of Rust workspace crates.
- **Owner Overlays**: Parses `CODEOWNERS` to map reviewers directly to impacted atlas nodes.

#### Validation and Self-Policing

- **Validation Profiles**: Three validation profiles (default, ci, strict).
- **Diagnostic Messages**: Clear, actionable diagnostic messages for validation failures.
- **Doctor Command**: Built-in drift detection (`dead_selector`, `orphan_node`, `stale_command`, `duplicate_ownership`).

#### Operational Workflows

- **Impact Analysis**: Review-time workflow tool that maps a diff to behavior, proof surfaces, and docs. Identifies uncovered changed paths.
- **Semantic Navigation (Why)**: Projects a short, readable proof chain for any node or path.
- **Query and Trace**: Search nodes by ID/pattern and trace relationships in both directions with configurable depth.

#### Rendering and review artifacts

- **JSON Output**: Machine-readable outputs with stable schemas for `build`, `doctor`, `why`, and `impacted` command paths.
- **Markdown Output**: Human-readable `atlas.md` and command projections.
- **GitHub Summary**: Optimized markdown projection (`--format gh-summary`) for CI summaries.
- **Review Packet**: `review-packet` format for compact review-ready output.

#### Adoption Scaffolding

- **Init Command**: Generates starter `atlas.toml` to bootstrap new repositories.
- **Scaffold Command**: Generates valid YAML stubs for scenarios, artifacts, and requirements.

### Core commands (operational order)

```bash
# Bootstrap a repository
atlasctl init
atlasctl scaffold scenario my-new-feature

# Build the atlas
atlasctl build [--out-dir DIR] [--repo-root PATH] [--config PATH]

# Daily health gate
atlasctl check [--profile default|ci|strict] [--format text|json|markdown|gh-summary|review-packet]
atlasctl doctor [--format text|json|markdown|gh-summary|review-packet]

# Daily review workflow
atlasctl impacted [--base main --head HEAD] [--paths file.rs] [--format text|json|markdown|gh-summary|review-packet]

# Proof chain lookup
atlasctl why <id-or-path> [--path] [--format text|json|markdown|gh-summary|review-packet]

# Query nodes
atlasctl query <needle> [--kind KIND]

# Trace relationships
atlasctl trace <start> [--direction outgoing|incoming|both] [--max-depth N]

# Export formats
atlasctl export --format json|markdown|gh-summary|review-packet [--out PATH]
```

### Installation

```bash
# From source
cargo build --release

# The binary will be at target/release/atlasctl
```
Note: `atlasctl` requires Rust 1.92 or later (Edition 2024).

### Test Coverage

- **Cross-layer coverage across all crates**
- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON, Markdown, and GH summaries
- **CLI integration tests**: End-to-end command validation for all workflows

### Self-Dogfooding

The project uses `atlasctl` against its own graph data with a clean-check pipeline and release gates.

### Known Limitations

1. **Rust-only**: Currently optimized for Rust-style repositories with `Cargo.toml` files
2. **Local-only**: No remote service integration or distributed discovery
3. **Manual metadata**: Requires explicit metadata declarations; does not infer from code
4. **Single format**: YAML fragments and markdown frontmatter only (no TOML or JSON fragments yet)
5. **No cross-tool integration**: No integrations with other tools or services

### Future Roadmap

The living roadmap and spec-completion plan now lives in [docs/tasks.md](docs/tasks.md) and is the source of truth for prioritization and closure criteria.

### Release Bar

Before tagging a release:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -p xtask -- mutants` (mutation testing against diff-critical paths)
- Path portability checks pass (no absolute or machine-local path leakage)
- `cargo run -p atlasctl-cli -- check --profile ci` (Zero-warning self-atlas)

### Versioning

- Bump schema only when the canonical JSON contract changes materially
- Document node kind, edge kind, or profile changes in ADRs
- Follow semantic versioning (MAJOR.MINOR.PATCH)

### License

MIT License - see [LICENSE](LICENSE) file for details.
