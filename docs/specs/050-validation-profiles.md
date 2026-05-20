# 050 - Validation Profiles

Profiles control severity and strictness for the same graph.

| Profile | Purpose |
|---|---|
| `default` | fast local feedback |
| `ci` | pre-merge checks |
| `strict` | release-grade controls |

## Rule examples

| Code | Meaning | Default (`default`) | `ci` | `strict` |
|---|---|---|---|---|
| `ATLAS001` | Duplicate node id | error | error | error |
| `ATLAS002` | Broken edge reference | error | error | error |
| `ATLAS003` | Duplicate path ownership | error | error | error |
| `ATLAS004` | Dead path selector | warning | warning | warning/error |
| `ATLAS005` | Absolute path leaked into artifact | warning | error | error |
| `ATLAS006` | Missing node role | warning | warning | error |
| `ATLAS007` | Requirement has no proof | warning | warning | error |
| `ATLAS008` | Scenario has no command | error | error | error |
| `ATLAS009` | Scenario has no proof artifact | warning | warning | error |
| `ATLAS010` | Changed path has no atlas coverage | warning | error | error |

Diagnostic shape:

```json
{
  "code": "ATLAS007",
  "severity": "warning",
  "profile": "default",
  "message": "Requirement has no proving scenario.",
  "node": "req:deterministic-atlas",
  "rule": "requirement-proof"
}
```

## Validation philosophy

- deterministic diagnostics
- stable rule IDs
- no hidden assumptions
- warnings can become errors only by profile policy

## Drift checks

Profiles may enforce:

- schema drift checks
- stale artifact paths
- uncovered-path and uncovered-crate checks
- command/reference consistency
