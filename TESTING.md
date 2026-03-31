# TESTING

## Verification stack

This repo follows a layered verification model.

### Types and codes

- unit tests
- property tests for stable parsing and ordering

### Core

- scenario tests for validation and trace semantics
- property tests for determinism and integrity
- mutation testing later on diff-critical paths

### Discovery

- fixture-repo tests
- malformed fragment tests
- frontmatter parsing tests

### Render

- snapshot and golden tests for `atlas.json` and `atlas.md`

### CLI

- smoke tests
- exit-code tests
- query and trace output tests

## Expected commands

```bash
cargo test --workspace
cargo run -p atlasctl-cli -- build
cargo run -p atlasctl-cli -- check --profile ci
```

## Golden discipline

`atlas.json` is the contract. If it changes intentionally:

1. update the schema if needed
2. update golden outputs
3. update docs and examples
