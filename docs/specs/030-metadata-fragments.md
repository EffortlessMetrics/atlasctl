# 030 - Metadata Fragments

Fragments are the primary source of graph declaration.

## Supported declarations

1. `*.atlas.yaml` fragments for explicit nodes and edges.
2. Markdown frontmatter for lightweight nodes embedded in documentation.
3. Discovery of Rust crates from `Cargo.toml` (implementation-defined).

## Fragment schema

```yaml
schema_version: 1

nodes:
  - id: req:deterministic-atlas
    kind: requirement
    role: behavior
    title: Deterministic atlas
    summary: Same input metadata produces stable output artifacts.

  - id: scen:build-emits-canonical-atlas
    kind: scenario
    role: proof
    title: Build emits canonical atlas
    summary: `build` compiles graph and artifact projections.

  - id: cmd:ci-fast
    kind: command
    role: command
    title: Fast CI
    command: cargo run -p xtask -- ci-fast

  - id: artifact:atlas-json
    kind: artifact
    role: artifact
    title: Canonical atlas JSON
    paths:
      owns:
        - ".atlas/atlas.json"

edges:
  - from: scen:build-emits-canonical-atlas
    kind: proves
    to: req:deterministic-atlas
  - from: scen:build-emits-canonical-atlas
    kind: runs_with
    to: cmd:ci-fast
  - from: cmd:ci-fast
    kind: emits
    to: artifact:atlas-json
```

## Fragment rules

- Empty fragment files are invalid.
- IDs must match `<kind>:<slug>`.
- Paths in fragments are normalized, repo-relative, and slash-separated.
- Edges are validated against known nodes and relation kinds.
- Duplicate IDs and overlapping ownership are validated per profile.

Path selectors should be explicit:

- `paths.owns`: responsibility.
- `paths.touches`: participation.

## Source-of-truth shape

Fragments should be stable enough to drive CI checks, PR review packets, and local evidence checks without inference.

The graph compiler treats:

- `nodes` as explicit assertions.
- `edges` as evidence and ownership relationships.
