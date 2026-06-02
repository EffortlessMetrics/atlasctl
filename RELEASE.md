# RELEASE

## Unreleased / Post-v0.1.0 Closeout Updates

### Lane v0.1.0 Closeout Validation

The source-of-truth stack proof-topology closeout follow-up includes:

- Review-packet UX hardening for changed-path, owner, and scope summary reporting.
- Machine-readable `review-packet` output (`--format json`) with `impact` envelope payload and `command: review-packet`.
- Path normalization/compatibility fixes for `why` and `impacted` path inputs.
- `why --path` now returns actionable guidance for no-match cases:
  - missing typo/missing path guidance,
  - existing orphan-path metadata coverage guidance,
  - safer recursive-touch matching defaults.
- Dogfood sample evidence added for local follow-up change window.
- Additional scope diagnostics for doc/generation boundary handling.

### Verified Evidence

The closeout changes were verified with:

- `rtk cargo fmt --all -- --check`
- `rtk cargo test --workspace` (197 tests)
- `rtk cargo run -p xtask -- ci-full`
- `rtk cargo run -p xtask -- docs-check`
- `rtk cargo run -p xtask -- schema --check`
- `rtk cargo run -p atlasctl-cli -- doctor --profile ci --repo-root .`
- `rtk cargo run -p atlasctl-cli -- impacted --base main --head HEAD --format review-packet --repo-root .`
- `rtk cargo run -p atlasctl-cli -- why --repo-root . --path crates/atlasctl-core/src/lib.rs`
- `rtk cargo run -p atlasctl-cli -- impacted --base HEAD~1 --head HEAD --format review-packet --repo-root .`
- `rtk cargo test -p atlasctl-cli --test cli_integration test_why_by_missing_path`

### Release Bar

- Keep existing release bar checks from v0.1.0.
- Keep release-note coverage for review-packet/impact semantics changes in `docs/handoffs`.
- Preserve deterministic outputs and protocol checks before any schema-facing release.

## Version 0.1.0

Release Date: 2026-04-16

### Overview

This is the v0.1.0 release of `atlasctl`, a local-first scenario and proof atlas compiler for Rust-style repositories. This release transforms the tool from a static graph builder into a full review-time proof navigator, offering a deterministic, queryable graph, and deep operational features like impact analysis and drift detection.

### Features Implemented

#### Core Functionality

- **Graph Model**: Stable graph model with nodes and edges representing requirements, ADRs, scenarios, fixtures, commands, artifacts, and workspace crates.
- **Deterministic Compilation**: Reproducible atlas generation from repository metadata with strict path normalization (forward slashes, repo-relative paths only).
- **Fragment Discovery**: YAML fragment discovery from `atlas/` directory.
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

#### Rendering

- **JSON Output**: Machine-readable outputs with stable schemas for `build`, `doctor`, `why`, and `impacted` commands.
- **Markdown Output**: Human-readable `atlas.md` with formatted sections and dossiers.
- **GitHub Summary**: Optimized markdown projection (`--format gh-summary`) for CI step summaries.

#### Adoption Scaffolding

- **Init Command**: Generates starter `atlas.toml` to bootstrap new repositories.
- **Scaffold Command**: Generates valid YAML stubs for scenarios, artifacts, and requirements.

### Commands

```bash
# Bootstrap a repository
atlasctl init
atlasctl scaffold scenario my-new-feature

# Build the atlas
atlasctl build [--out-dir DIR] [--repo-root PATH] [--config PATH]

# Check validation and drift
atlasctl check [--profile default|ci|strict] [--format text|json|gh-summary]
atlasctl doctor [--format text|json|gh-summary]

# Review-time impact analysis
atlasctl impacted [--base main --head HEAD] [--paths file.rs] [--format text|json|gh-summary]

# Get a proof chain
atlasctl why <id-or-path> [--path] [--format text|json|gh-summary]

# Query nodes
atlasctl query <needle> [--kind KIND]

# Trace relationships
atlasctl trace <start> [--direction outgoing|incoming|both] [--max-depth N]

# Export formats
atlasctl export --format json|markdown|gh-summary [--out PATH]
```

### Installation

```bash
# From source
cargo build --release

# The binary will be at target/release/atlasctl
```
Note: `atlasctl` requires Rust 1.92 or later (Edition 2024).

### Test Coverage

- **197 tests** across all crates
- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON, Markdown, and GH summaries
- **CLI integration tests**: End-to-end command validation for all workflows

### Self-Dogfooding

The project successfully uses `atlasctl` to track its own behavior:

- **45 nodes** representing requirements, scenarios, ADRs, commands, artifacts, support tiers, policy ledgers, plans, and claims
- **61 edges** representing relationships across the full proof-topology stack
- **0 diagnostics** - clean validation

### Known Limitations

1. **Rust-only**: Currently optimized for Rust-style repositories with `Cargo.toml` files
2. **Local-only**: No remote service integration or distributed discovery
3. **Manual metadata**: Requires explicit metadata declarations; does not infer from code
4. **Primary format**: YAML fragments and markdown frontmatter for repository truth declarations (policy/atlas config uses TOML where appropriate)
5. **No cross-tool integration**: No integrations with other tools or services

### Future Roadmap

- Advanced diffing and atlas snapshot comparison
- Enhanced query syntax (filters, sorting, projections)
- Additional fragment formats (TOML, JSON)
- IDE integration support
- Performance optimizations for large repositories
- Additional output formats (HTML, Graphviz, DOT)

### Release Bar

Before tagging a release:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Path portability checks pass (no absolute or machine-local path leakage)
- `cargo run -p atlasctl-cli -- check --profile ci` (Zero-warning self-atlas)

### Versioning

- Bump schema only when the canonical JSON contract changes materially
- Document node kind, edge kind, or profile changes in ADRs
- Follow semantic versioning (MAJOR.MINOR.PATCH)

### License

MIT License - see [LICENSE](LICENSE) file for details.
