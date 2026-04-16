# Building atlasctl

## Prerequisites

- Rust 1.92 or later (Edition 2024)
- Cargo
- system Git

## Build

```bash
cargo build --release
cargo test --workspace
cargo run -p atlasctl-cli -- build
```

## Operator examples

```bash
# Initialize and scaffold
cargo run -p atlasctl-cli -- init
cargo run -p atlasctl-cli -- scaffold scenario my-scenario

# Verify your repo structure
cargo run -p atlasctl-cli -- doctor
cargo run -p atlasctl-cli -- check --profile ci

# Review impact and proof chains
cargo run -p atlasctl-cli -- impacted --base main --format gh-summary
cargo run -p atlasctl-cli -- why scen:build-emits-canonical-atlas

# Navigate the graph
cargo run -p atlasctl-cli -- query scen:build-emits-canonical-atlas
cargo run -p atlasctl-cli -- trace req:deterministic-atlas
cargo run -p atlasctl-cli -- export --format markdown
```
