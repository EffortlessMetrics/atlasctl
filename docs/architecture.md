---
atlas:
  id: guide:architecture
  kind: guide
  title: Architecture
  documents:
    - req:deterministic-atlas
    - req:queryable-proof-topology
---
# Architecture

`atlasctl` is a deterministic compiler for review proof topology.

## Composition layers

### Core

`atlasctl-types` and `atlasctl-core` hold:

- node and edge types
- graph assembly
- validation and diagnostics
- query, trace, and impact semantics

### Adapters and services

- `atlasctl-discover-fs`: discovery and parsing adapters (filesystem, frontmatter, Cargo workspace).
- `atlasctl-app`: command-level orchestration over typed interfaces.
- `atlasctl-render`: deterministic projections (`atlas.json`, `atlas.md`, summaries, review packets).
- `atlasctl-cli`: CLI entrypoint and output wiring.
- `atlasctl-fixtures`: local fixture sources used for behavior tests.

## Execution flow

1. Compile phase: discover metadata from `atlas.toml`, `.atlas.yaml` fragments, markdown frontmatter, and workspace facts.
2. Build phase: assemble nodes, edges, ownership, and validation context.
3. Verify phase: apply profile settings and emit structured diagnostics.
4. Review phase: produce proof-oriented responses for `impacted`, `doctor`, and `why`.
5. Serve phase: execute query/trace/impact/why commands with deterministic ranking and stable paths.

## Graph as contract

All runtime behavior is derived from the graph contract:

- stable identifiers (`<kind>:<slug>`)
- deterministic ordering for output lists
- explicit edge types with ownership/participation semantics (`owns` / `touches`)
- role-aware validation by explicit node role

This avoids hidden inference and keeps output behavior predictable across machines.
