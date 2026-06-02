# RELEASE

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

- **161 tests** across all crates
- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON, Markdown, and GH summaries
- **CLI integration tests**: End-to-end command validation for all workflows

### Self-Dogfooding

The project successfully uses `atlasctl` to track its own behavior:

- **42 nodes** representing requirements, scenarios, ADRs, commands, artifacts, support tiers, policy ledgers, plans, and claims
- **56 edges** representing relationships across the full proof-topology stack
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
