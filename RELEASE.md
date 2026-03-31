# RELEASE

## Version 0.1.0

Release Date: 2026-03-31

### Overview

This is the initial release of `atlasctl`, a local-first scenario and proof atlas compiler for Rust-style repositories. This release provides a stable foundation for compiling repository metadata into a deterministic, queryable graph.

### Features Implemented

#### Core Functionality

- **Graph Model**: Stable graph model with nodes and edges representing requirements, ADRs, scenarios, fixtures, commands, artifacts, and workspace crates
- **Deterministic Compilation**: Reproducible atlas generation from repository metadata
- **Fragment Discovery**: YAML fragment discovery from `atlas/` directory
- **Frontmatter Parsing**: Markdown frontmatter parsing for embedded metadata
- **Workspace Crate Discovery**: Automatic discovery of Rust workspace crates

#### Validation

- **Validation Profiles**: Three validation profiles (default, ci, strict)
- **Diagnostic Messages**: Clear, actionable diagnostic messages for validation failures
- **Reference Validation**: Validation of node-to-node references and edge integrity

#### Query and Trace

- **Query API**: Search nodes by ID pattern and kind
- **Trace API**: Trace relationships in both directions with configurable depth
- **Deterministic Ordering**: Stable, predictable output ordering for all operations

#### Rendering

- **JSON Output**: Machine-readable `atlas.json` with stable schema
- **Markdown Output**: Human-readable `atlas.md` with formatted sections
- **Schema Definition**: JSON schema for validation and tooling integration

#### CLI

- **Build Command**: Compile atlas from repository metadata
- **Check Command**: Validate atlas against profiles
- **Query Command**: Search and query nodes
- **Trace Command**: Trace relationships between nodes
- **Export Command**: Export atlas in various formats
- **Exit Codes**: Standardized exit codes for automation

### Commands

```bash
# Build the atlas
atlasctl build [--out-dir DIR] [--repo-root PATH] [--config PATH]

# Check validation
atlasctl check [--profile default|ci|strict] [--format text|json]

# Query nodes
atlasctl query <needle> [--kind KIND] [--repo-root PATH]

# Trace relationships
atlasctl trace <start> [--direction outgoing|incoming|both] [--max-depth N]

# Export formats
atlasctl export --format json|markdown [--out PATH]
```

### Installation

```bash
# From source
cargo build --release

# The binary will be at target/release/atlasctl
```

### Documentation

- [Architecture](docs/architecture.md) - System architecture and design principles
- [Design](docs/design.md) - Detailed design decisions
- [Requirements](docs/requirements.md) - Project requirements
- [Testing Strategy](docs/testing-strategy.md) - Testing methodology
- [Metadata Conventions](docs/metadata-conventions.md) - How to write atlas metadata
- [Mission and Vision](docs/mission-and-vision.md) - Project goals
- [Non-goals](docs/non-goals.md) - What this project explicitly does not do
- [Tasks](docs/tasks.md) - Task breakdown and tracking
- [ADRs](docs/adr/) - Architecture Decision Records

### Test Coverage

- **75 tests** across all crates
- **BDD tests**: Scenario-based tests validating graph semantics
- **Property tests**: Proptest-based tests for determinism and integrity
- **Golden file tests**: Snapshot tests for JSON and Markdown output
- **CLI integration tests**: End-to-end command validation

### Self-Dogfooding

The project successfully uses `atlasctl` to track its own behavior:

- **31 nodes** representing requirements, scenarios, ADRs, fixtures, commands, and artifacts
- **24 edges** representing relationships between nodes
- **0 diagnostics** - clean validation

### Known Limitations

1. **Rust-only**: Currently optimized for Rust-style repositories with `Cargo.toml` files
2. **Local-only**: No remote service integration or distributed discovery
3. **Manual metadata**: Requires explicit metadata declarations; does not infer from code
4. **Single format**: YAML fragments and markdown frontmatter only (no TOML or JSON fragments yet)
5. **No impact analysis**: Cannot yet analyze the impact of changes across the graph
6. **No cross-tool integration**: No integration with other tools or services

### Future Roadmap

#### Short-term (Post-0.1.0)

- Additional fragment formats (TOML, JSON)
- Enhanced query syntax (filters, sorting)
- More validation rules and profiles
- Performance optimizations for large repositories

#### Medium-term

- Impact analysis capabilities
- Cross-repo atlas composition
- Additional output formats (HTML, Graphviz)
- Plugin system for custom discovery adapters

#### Long-term

- Cross-tool integration (CI/CD, documentation generators)
- Remote service support for distributed teams
- AI-assisted metadata suggestions
- Web-based atlas visualization

### Migration Guide

This is the initial release, so no migration is needed. Future releases will document any breaking changes and migration paths.

### Release Bar

Before tagging a release:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -p atlasctl-cli -- build`
- `cargo run -p atlasctl-cli -- check --profile ci`

### Versioning

- Bump schema only when the canonical JSON contract changes materially
- Document node kind, edge kind, or profile changes in ADRs
- Keep output ordering stable
- Follow semantic versioning (MAJOR.MINOR.PATCH)

### Artifacts

The first public artifact is source plus tags. Prebuilt binaries can come later with `cargo-dist`.

### Contributing

Contributions are welcome! See [AGENTS.md](AGENTS.md) for guidelines on where to make changes and how to structure contributions.

### License

MIT License - see [LICENSE](LICENSE) file for details.

### Acknowledgments

This project was developed to address the challenge of making repository knowledge explicit and queryable, enabling better collaboration between humans and AI agents.
