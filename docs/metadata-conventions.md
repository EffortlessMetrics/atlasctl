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

## External onboarding: minimum metadata for usable `why` output

To get meaningful `why` chains and impact surfaces in a repository that is already
using Atlas, add at least one scenario node tied to changed paths plus a root
`atlas.toml` that includes your metadata roots.

Minimal `atlas.toml`:

```toml
schema_version = 1

[discovery]
roots = ["atlas", "docs", "README.md"]
ignore = ["target", ".git", "node_modules"]
```

Minimal metadata file (for example `atlas/repo-truth.atlas.yaml`):

```yaml
nodes:
  - id: req:repo-truth
    kind: requirement
    title: Repository behavior is documented
    owns:
      - docs
    touches:
      - crates

  - id: scen:repo-truth-surface
    kind: scenario
    title: Repo changes are explainable
    owns:
      - crates
      - docs
    proves:
      - req:repo-truth

  - id: cmd:ci-fast
    kind: command
    title: Fast CI

  - id: artifact:atlas-json
    kind: artifact
    title: Atlas graph artifact

edges:
  - from: scen:repo-truth-surface
    kind: proves
    to: req:repo-truth

  - from: scen:repo-truth-surface
    kind: runs_with
    to: cmd:ci-fast

  - from: scen:repo-truth-surface
    kind: emits
    to: artifact:atlas-json
```

With this in place, `atlasctl why --path <path>`, `doctor`, and `impacted` can
return path-aware proof links instead of only discovery-only structural warnings.
