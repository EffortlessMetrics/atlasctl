# 060 - Artifact Protocol

Machine-facing outputs are contracts, not convenience prints.

## Canonical artifact set

- `atlas.json` (`schema_version`, `repo`, `graph`, `diagnostics`, `metrics`)
- `doctor.json` (`schema_version`, `status`, `diagnostics`, `gaps`)
- `why.json` (`schema_version`, `query`, `result`, `gaps`)
- `impact.json` (`schema_version`, `changed_paths`, `impacted_nodes`, `gaps`, `diagnostics`)
- Optional projections: `atlas.md`, `*.gh-summary.md`, `*.review-packet.md`

Each machine-facing payload is a protocol contract:

- explicit `schema_version`
- stable field names
- deterministic ordering
- repo-relative slash-normalized paths
- schema checks and compatibility policy

## Command outputs

| Command | Primary payload |
|---|---|
| `build` | graph contract (`atlas.json`) + projections |
| `check` | graph + diagnostics |
| `doctor` | graph + diagnostics |
| `why` | proof response |
| `impacted` | impact response |
| `export` | requested projection format |

## Formats

- `json` — schema-validated payload.
- `markdown` — human-readable projection.
- `gh-summary` — compact CI summary.
- `review-packet` — compact review packet.

## Compatibility

- No protocol-breaking output field removals or renames without ADR and migration note.
- Optional fields are additive and should remain backward-compatible.
- Golden tests should verify `json` and markdown projections.
- Add no-absolute-path and slash-normalization tests before schema-locking.

## Canonical schemas

Generated under `schemas/`:

- `atlas.schema.json`
- `impact.schema.json`
- `why.schema.json`
- `doctor.schema.json` (currently graph-shaped)

## Protocol invariants

- `schema_version` is explicit.
- all file paths are repo-relative and slash-normalized.
- output ordering is stable.
- no absolute local path leakage.
- adding optional fields is non-breaking; removing or renaming is breaking.

## Canonical JSON shape (abridged)

```json
{
  "schema_version": 1,
  "repo": {
    "id": "atlasctl",
    "name": "atlasctl"
  },
  "graph": {
    "nodes": [],
    "edges": []
  },
  "diagnostics": [],
  "metrics": {
    "node_count": 0,
    "edge_count": 0
  }
}
```
