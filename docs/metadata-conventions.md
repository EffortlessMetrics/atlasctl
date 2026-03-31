---
atlas:
  id: guide:metadata-conventions
  kind: guide
  title: Metadata conventions
  documents:
    - req:queryable-proof-topology
---
# Metadata conventions

## Root config

The root `atlas.toml` controls discovery and validation defaults.

## Fragments

Use `*.atlas.yaml` for explicit metadata.

A fragment may declare:

- nodes
- edges

## Markdown frontmatter

Markdown may declare atlas metadata using frontmatter:

```yaml
---
atlas:
  id: adr:0001-product-scope
  kind: adr
  explains:
    - req:deterministic-atlas
---
```

## IDs

Stable IDs are mandatory.

Examples:

- `req:deterministic-atlas`
- `scen:build-emits-canonical-atlas`
- `cmd:ci-fast`
- `artifact:atlas-json`
