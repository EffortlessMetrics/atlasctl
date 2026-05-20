# Artifact protocol

Use `docs/specs/060-artifact-protocol.md` for the contract.

This document summarizes production behavior.

## Output matrix

| Command | JSON | Markdown | gh-summary | review-packet |
|---|---|---|---|---|
| `build` | `atlas.json` | `atlas.md` | n/a | n/a |
| `check` / `doctor` | graph contract + diagnostics | rendered projection | review summary | review packet |
| `why` | `WhyResponse` | rendered proof chain | compact review prose | review packet |
| `impacted` | `ImpactResponse` | impacted projection | compact review prose | review packet |
| `query` / `trace` | text (current) | text (current) | n/a | n/a |
| `export` | configurable | configurable | configurable | configurable |

## Invariants

- All machine outputs are schema-backed.
- path values are repo-relative and slash-normalized.
- diagnostics are attached when validation is requested.
- output ordering is stable and deterministic.

## Schema governance

Current schema artifacts are in `schemas/`:

- `atlas.schema.json`
- `impact.schema.json`
- `why.schema.json`
- `doctor.schema.json`

Regenerate after contract changes:

```bash
cargo run -p xtask -- schema
```
