# 010 - Graph Model

This model is the contract that every command derives from.

## Nodes

Node identity and role are explicit:

- `id`: `<kind>:<slug>` where:
  - `<kind>` is one of `requirement`, `scenario`, `adr`, `guide`, `fixture`, `command`, `artifact`, `crate`, `document`
  - `<slug>` is lower-case, `-`/`_` permitted.
- `kind`: domain category that maps to validation expectations.
- `role`: validation and contract role that drives proof policy.
- `status` (optional): `active`, `deprecated`, or similar lifecycle marker.
- `owns` / `touches` selectors are optional and should be repo-relative.

### Role mapping

| Kind | Role |
|---|---|
| `requirement` | `behavior` |
| `scenario`, `fixture` | `proof` |
| `adr`, `guide`, `document` | `document` |
| `command` | `command` |
| `artifact` | `artifact` |
| `crate` | `infra` |

### Example node

```yaml
id: req:deterministic-atlas
kind: requirement
role: behavior
title: Deterministic atlas
summary: Build stable graph artifacts from the same inputs.
status: active
owns:
  - core/graph
tags:
  - determinism
  - protocol
```

## Edges

| Edge kind (current field `kind`, documented as `relation`) | Meaning |
|---|---|
| `proves` | Scenario/proof node demonstrates requirement |
| `documents` | Node documents the target |
| `explains` | ADR explains requirement/design |
| `uses_fixture` | Scenario uses fixture |
| `runs_with` | Scenario is run with command |
| `exercises` | Scenario touches implementation node (often a crate) |
| `emits` | Command emits artifact |
| `touches` | Scenario or node participates in another artifact |
| `depends_on` | Command dependency / prerequisite |
| `belongs_to` | Artifact/component association |
| `supports` | Evidence support relationship |
| `owns` | Path ownership relationship |

## Ownership vs participation

Path relationships must be explicit and separated:

- `owns` means a node is responsible for that path/surface.
- `touches` means a node participates in, verifies, or is affected by the path.

Rules:

- `duplicate owns => error`
- `overlapping touches => allowed`
- Uncovered changed paths produce warning or error by profile.

## Stability rules

- Stable ordering by canonical key (id, kind, relation, path).
- Repo-relative, slash-normalized paths.
- No local absolute paths in artifact payloads.
- Deterministic rendering so repeated builds from the same inputs produce equivalent outputs.
