---
atlas:
  id: guide:metadata-conventions
  kind: guide
  title: Metadata conventions
  documents:
    - req:deterministic-atlas
---
# Metadata conventions

All metadata follows explicit, typed IDs and machine-parseable provenance.

## Root configuration

`atlas.toml` controls:

- repository identity,
- discovery roots and ignore patterns,
- profile defaults and overrides,
- path and parse behavior.

## Supported sources

1. `atlas` fragments (`*.atlas.yaml`) for bulk declarations.
2. Markdown frontmatter for inline docs and lightweight nodes.
3. Cargo workspace metadata (`Cargo.toml`) for crate discovery.

## YAML fragment shape

Each fragment file must declare at least one `node` or `edge` block.

- IDs must use `<kind>:<slug>` and are validated against kind-specific grammar.
- Unknown targets are reported as broken references.
- Empty fragments are rejected.
- Node role must be set explicitly in `*.atlas.yaml` declarations.
- Legacy/adaptive discovery paths (for example, crate discovery) may still derive role from kind, but the long-term contract is explicit declaration.

## Markdown frontmatter

Markdown metadata must include atlas identity:

```yaml
---
atlas:
  id: adr:0001-product-scope
  kind: adr
  role: document
  title: Product scope and non-goals
  explains:
    - req:deterministic-atlas
  documents:
    - scen:build-emits-canonical-atlas
---
```

Supported atlas keys:

- `id`: stable atlas ID
- `kind`: node kind (`guide`, `requirement`, `scenario`, `adr`, `fixture`, `command`, `artifact`, `crate`, `document`)
- `role`: node role (`behavior`, `proof`, `document`, `artifact`, `command`, `infra`)
- `title`: display label
- `summary`: one-line intent
- `explains`: dependencies this node explains
- `documents`: evidence documents
- `proves`, `touches`, `owns`, `runs_with`, `exercises`, `documents`: optional relationship keys

### Ownership vs participation

- `owns` is exclusive or near-exclusive ownership.
- `touches` is permissive participation.
- `paths` (frontmatter) can be used as legacy shorthand for `owns`.

## ID conventions

Examples:

- `req:deterministic-atlas`
- `scen:build-emits-canonical-atlas`
- `cmd:ci-fast`
- `artifact:atlas-json`

IDs must remain stable once published in public docs and consumed by tests.

## Node roles by kind

| Kind | Role |
|---|---|
| `requirement` | `behavior` |
| `adr`, `guide`, `document` | `document` |
| `scenario`, `fixture` | `proof` |
| `command` | `command` |
| `artifact` | `artifact` |
| `crate` | `infra` |
