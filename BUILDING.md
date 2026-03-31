# Building atlasctl

## Prerequisites

- Rust stable
- Cargo
- system Git

## Build

```bash
cargo build
cargo test --workspace
cargo run -p atlasctl-cli -- build
```

## Operator examples

```bash
cargo run -p atlasctl-cli -- check --profile ci
cargo run -p atlasctl-cli -- query scen:build-emits-canonical-atlas
cargo run -p atlasctl-cli -- trace req:deterministic-atlas
cargo run -p atlasctl-cli -- export --format markdown
```
