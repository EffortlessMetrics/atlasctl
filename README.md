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

- Rust 1.75 or later
- Cargo (included with Rust)

## Usage

### Build the atlas

Compile the repository's metadata into `atlas.json` and `atlas.md`:

```bash
atlasctl build
```

By default, outputs are written to `.atlas/` directory. Use `--out-dir` to customize:

```bash
atlasctl build --out-dir ./output
```

### Check validation

Validate the atlas against a profile:

```bash
# Default profile
atlasctl check

# CI profile (stricter validation)
atlasctl check --profile ci

# Strict profile
atlasctl check --profile strict

# Output as JSON
atlasctl check --format json
```

### Query nodes

Search for nodes by ID or pattern:

```bash
# Query by scenario ID
atlasctl query scen:build-emits-canonical-atlas

# Query with specific kind filter
atlasctl query "build" --kind scenario

# Query by requirement ID
atlasctl query req:deterministic-atlas
```

### Trace relationships

Trace relationships from a starting node:

```bash
# Trace outgoing edges (what this node references)
atlasctl trace req:deterministic-atlas --direction outgoing

# Trace incoming edges (what references this node)
atlasctl trace req:deterministic-atlas --direction incoming

# Trace both directions (default)
atlasctl trace req:deterministic-atlas --direction both

# Limit trace depth
atlasctl trace req:deterministic-atlas --max-depth 3
```

### Export formats

Export the atlas in specific formats:

```bash
# Export as JSON
atlasctl export --format json

# Export as Markdown
atlasctl export --format markdown

# Specify output file
atlasctl export --format json --out ./my-atlas.json
```

### Common options

All commands support these common options:

```bash
# Specify repository root (default: current directory)
atlasctl build --repo-root /path/to/repo

# Use custom config file
atlasctl build --config /path/to/atlas.toml

# Set validation profile
atlasctl check --profile ci
```

## Repository shape

The workspace is split into:

- `atlasctl-core` for graph assembly, validation, query, and trace
- `atlasctl-discover-fs` for repo-local discovery
- `atlasctl-render` for JSON and Markdown projections
- `atlasctl-app` for orchestration
- `atlasctl-cli` for the operator surface
- `atlasctl-types` for shared type definitions
- `atlasctl-codes` for exit codes and error handling
- `atlasctl-ports` for trait definitions
- `atlasctl-fixtures` for test fixtures

The atlas uses explicit metadata first. It does not try to infer the repo's meaning from arbitrary code.

## Current status

This repository is a vertical slice:

- graph model
- deterministic compilation
- fragment and frontmatter discovery
- workspace crate discovery
- validation profiles
- query and trace
- JSON and Markdown output
- docs and fixture repos

That is enough to dogfood on this repo and to extend later with impact analysis and cross-tool integration.

## Documentation

- [Architecture](docs/architecture.md) - System architecture and design principles
- [Design](docs/design.md) - Detailed design decisions
- [Requirements](docs/requirements.md) - Project requirements
- [Testing Strategy](docs/testing-strategy.md) - How testing is structured
- [Metadata Conventions](docs/metadata-conventions.md) - How to write atlas metadata
- [Mission and Vision](docs/mission-and-vision.md) - Project goals
- [Non-goals](docs/non-goals.md) - What this project explicitly does not do
- [Tasks](docs/tasks.md) - Task breakdown and tracking
- [ADRs](docs/adr/) - Architecture Decision Records

## Development

### Development setup

```bash
# Clone the repository
git clone https://github.com/EffortlessMetrics/atlasctl.git
cd atlasctl

# Install development dependencies (optional, for xtask)
cargo install --path xtask

# Run tests
cargo test --workspace

# Run CI checks
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full
```

### Development commands

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Build all crates
cargo build --workspace

# Build release binary
cargo build --release

# Run tests
cargo test --workspace

# Run specific test
cargo test --package atlasctl-core test_name

# Update golden files (after intentional changes)
cargo test --package atlasctl-core -- --accept

# Run smoke tests
cargo run -p xtask -- smoke

# Check documentation
cargo run -p xtask -- docs-check
```

### Self-dogfooding

The project uses `atlasctl` to track its own behavior:

```bash
# Build the atlas for this repo
cargo run -p atlasctl-cli -- build

# Check validation
cargo run -p atlasctl-cli -- check --profile ci

# Query the atlas
cargo run -p atlasctl-cli -- query scen:build-emits-canonical-atlas
```

## Contributing

This repository is shaped for delegation. See [AGENTS.md](AGENTS.md) for guidelines on where to make changes.

## License

MIT License - see [LICENSE](LICENSE) file for details.
