# 070 - Review Impact

`impacted` answers, on every commit, the exact review surface that changed.

## Input

- `--base` + `--head` (or default repo default)
- `--paths` explicit path list

## Response shape

```json
{
  "schema_version": 1,
  "changed_paths": [],
  "requirements": [],
  "scenarios": [],
  "crates": [],
  "impacted_nodes": [],
  "proof_commands": [],
  "artifacts": [],
  "owners": [],
  "docs": [],
  "gaps": [],
  "diagnostics": []
}
```

## Coverage classes

- `owned`: changed paths map to node ownership.
- `touched`: changed paths map to participating proof/infra nodes.
- `proved_by`: changed behavior is tied to proof scenarios.
- `documents`: changed path affects narrative/proof docs.
- `uncovered`: changed path has no atlas match under active profile.

## Review expectations

- Changed paths should produce stable and explainable impact.
- Uncovered paths require explicit follow-up:
  - add ownership,
  - add participation, or
  - record review acceptance.
- Output must be stable enough for `gh-summary` and `review-packet` to compare across runs.
