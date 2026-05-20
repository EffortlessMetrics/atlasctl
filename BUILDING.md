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
# Onboard and set defaults
cargo run -p atlasctl-cli -- init
cargo run -p atlasctl-cli -- scaffold scenario my-scenario

# Verify health first
cargo run -p atlasctl-cli -- doctor
cargo run -p atlasctl-cli -- check --profile ci

# Map your change to proof surface
cargo run -p atlasctl-cli -- impacted --base main --head HEAD

# Review impact and proof chains
cargo run -p atlasctl-cli -- why --path crates/atlasctl-core/src/lib.rs
cargo run -p atlasctl-cli -- why req:deterministic-atlas

# Export review payloads
cargo run -p atlasctl-cli -- export --format review-packet --out .atlas/atlas.review-packet.md
cargo run -p atlasctl-cli -- export --format gh-summary --out .atlas/atlas.gh-summary.md

# Navigate the graph
cargo run -p atlasctl-cli -- query scen:build-emits-canonical-atlas
cargo run -p atlasctl-cli -- trace req:deterministic-atlas
```
