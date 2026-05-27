# AGENTS

This repository is shaped for delegation.

## What matters most

1. Keep the graph model stable.
2. Prefer explicit metadata over inference.
3. Preserve deterministic ordering in every output.
4. Put filesystem and parsing mess in adapters, not in `atlasctl-core`.
5. Treat `atlas.json` as the canonical contract.

## Where to make changes

- Graph semantics: `crates/atlasctl-core`
- IDs, node kinds, edge kinds, config, diagnostics: `crates/atlasctl-types` (formerly `atlasctl-codes`)
- Filesystem scanning and parsing: `crates/atlasctl-discover-fs`
- Rendering: `crates/atlasctl-render`
- CLI behavior: `crates/atlasctl-cli`

## Guardrails

- Do not add remote-service assumptions.
- Do not add magical inference from test code.
- Do not add HTML before JSON and Markdown stay stable.
- Do not add impact analysis before the graph contract settles.
- Do not change ID grammar or node/edge kinds without an ADR.

## Proof surface

A good change usually updates one or more of:

- scenario tests in `core`
- fixture repo coverage
- golden JSON/Markdown output
- docs and ADRs if the public contract changes

## Commands

Use the `xtask` flow once the toolchain is available:

```bash
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full
cargo run -p xtask -- smoke
cargo run -p xtask -- docs-check
```
