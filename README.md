# atlasctl

`atlasctl` is a local-first scenario and proof atlas compiler for Rust-style repositories.

It compiles declared metadata about:

- requirements
- ADRs
- scenarios
- fixtures
- commands
- artifacts
- workspace crates

into a deterministic, queryable atlas.

The output is a stable `atlas.json` plus a human-readable `atlas.md`.

## Why it exists

Good repos hide too much meaning in:

- test names
- fixture directories
- ADR prose
- maintainer memory
- CI scripts

That makes delegation expensive.

`atlasctl` turns that implicit topology into an explicit graph so humans and agents can answer:

- what behavior matters here?
- what proves it?
- what fixture models it?
- what artifact should I inspect?
- what docs explain why it exists?

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/EffortlessMetrics/atlasctl.git
cd atlasctl

# Build the release binary
cargo build --release

# The binary will be available at target/release/atlasctl.exe (Windows)
# or target/release/atlasctl (Unix-like systems)
```

### Prerequisites

- Rust 1.92 or later
- Cargo (included with Rust)

## Usage

### Onboarding a repo

Bootstrap a new atlas for a repository:

```bash
# Initialize atlas.toml
atlasctl init

# Scaffold a new scenario
atlasctl scaffold scenario my-new-feature
```

### Build the atlas

Compile the repository's metadata into `atlas.json` and `atlas.md`:

```bash
atlasctl build
```

By default, outputs are written to `.atlas/` directory. Use `--out-dir` to customize:

```bash
atlasctl build --out-dir ./output
```

### Self-policing with `doctor`

Check for atlas drift and maintenance issues:

```bash
# Run diagnostics
atlasctl doctor

# Output as JSON for CI integration
atlasctl doctor --format json
```

### Query and Trace

Navigate the atlas graph:

```bash
# Search for nodes
atlasctl query "build" --kind scenario

# Trace relationships
atlasctl trace req:deterministic-atlas --direction outgoing

# Get a "Why" proof chain for a node or path
atlasctl why scen:example-build
atlasctl why --path crates/engine/src/lib.rs
```

### Review-time impact analysis

Map a diff to behavior and proof surfaces:

```bash
# Impact of local changes vs main
atlasctl impacted --base main --head HEAD

# Impact of specific paths
atlasctl impacted --paths crates/foo/src/lib.rs docs/adr/0002.md

# CI summary output
atlasctl impacted --format gh-summary
```

### Export formats

Export the atlas in specific formats:

```bash
# Export as JSON
atlasctl export --format json

# Export as GitHub Step Summary
atlasctl export --format gh-summary
```

## Repository shape

The workspace is split into:

- `atlasctl-core` for graph assembly, validation, query, and trace
- `atlasctl-discover-fs` for repo-local discovery and git integration
- `atlasctl-render` for JSON, Markdown, and CI-optimized projections
- `atlasctl-app` for orchestration
- `atlasctl-cli` for the operator surface
- `atlasctl-types` for shared type definitions
- `atlasctl-codes` for exit codes and error handling
- `atlasctl-ports` for trait definitions
- `atlasctl-fixtures` for test fixtures

The atlas uses explicit metadata first. It does not try to infer the repo's meaning from arbitrary code.

## Current status

This repository provides a complete operational core:

- deterministic graph model
- filesystem discovery with frontmatter support
- owner propagation from `CODEOWNERS`
- review-time impact analysis
- drift detection (`doctor`)
- semantic navigation (`why`)
- CI-optimized outputs (GitHub summaries)
- scaffolding tools (`init`, `scaffold`)

## Documentation

- [Architecture](docs/architecture.md) - System architecture and design principles
- [Design](docs/design.md) - Detailed design decisions
- [Requirements](docs/requirements.md) - Project requirements
- [Testing Strategy](docs/testing-strategy.md) - How testing is structured
- [Metadata Conventions](docs/metadata-conventions.md) - How to write atlas metadata
- [ADRs](docs/adr/) - Architecture Decision Records

## Development

### Development setup

```bash
# Run tests
cargo test --workspace

# Run CI checks
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full

# Run mutation testing
cargo run -p xtask -- mutants
```

### Self-dogfooding

The project uses `atlasctl` to track its own behavior:

```bash
# Build the atlas for this repo
cargo run -p atlasctl-cli -- build

# Check validation
cargo run -p atlasctl-cli -- check --profile ci

# Analyze impact of your changes
cargo run -p atlasctl-cli -- impacted --base main
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
