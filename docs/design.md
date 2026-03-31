# Design

`atlasctl` is a graph compiler, not a repo oracle.

## Architectural center

The pure center owns:

- typed nodes and edges
- graph assembly
- validation
- query
- trace
- diagnostics

## Adapters

Adapters own:

- config loading
- filesystem discovery
- YAML and frontmatter parsing
- Cargo workspace discovery
- output rendering

## Canonical artifact

The source of truth is `atlas.json`.

Markdown is a derived view.

## Metadata strategy

Prefer:

- `atlas.toml`
- `*.atlas.yaml`
- Markdown frontmatter
- Cargo workspace facts

Avoid magical inference from arbitrary code.
