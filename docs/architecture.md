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

`atlasctl` is a graph compiler with a hexagonal shape.

## Center

The pure center owns:

- node and edge types
- graph assembly
- validation
- query
- trace
- diagnostics

## Edges

Adapters own:

- reading `atlas.toml`
- scanning the filesystem
- parsing atlas fragments
- reading Markdown frontmatter
- discovering workspace crates
- rendering JSON and Markdown

## Crates

- `atlasctl-types`
- `atlasctl-core`
- `atlasctl-app`
- `atlasctl-discover-fs`
- `atlasctl-render`
- `atlasctl-cli`

Rendering is a leaf concern. The CLI is the composition root.
