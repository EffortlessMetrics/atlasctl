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

## Commands

```bash
cargo run -p atlasctl-cli -- build
cargo run -p atlasctl-cli -- check --profile ci
cargo run -p atlasctl-cli -- query scen:build-emits-canonical-atlas
cargo run -p atlasctl-cli -- trace req:deterministic-atlas
cargo run -p atlasctl-cli -- export --format json
```

## Repository shape

The workspace is split into:

- `atlasctl-core` for graph assembly, validation, query, and trace
- `atlasctl-discover-fs` for repo-local discovery
- `atlasctl-render` for JSON and Markdown projections
- `atlasctl-app` for orchestration
- `atlasctl-cli` for the operator surface

The atlas uses explicit metadata first. It does not try to infer the repo’s meaning from arbitrary code.

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
