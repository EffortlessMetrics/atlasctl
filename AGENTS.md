# AGENTS

This repository is shaped for delegation.

## What matters most

1. Keep the graph model stable.
2. Prefer explicit metadata over inference.
3. Preserve deterministic ordering in every output.
4. Put filesystem and parsing mess in adapters, not in `atlasctl-core`.
5. Treat `atlas.json` as the canonical contract.

## Where to make changes

- Graph semantics: `crates/atlasctl-core`
- IDs, node kinds, edge kinds, config, diagnostics: `crates/atlasctl-types`
- Filesystem scanning and parsing: `crates/atlasctl-discover-fs`
- Rendering: `crates/atlasctl-render`
- CLI behavior: `crates/atlasctl-cli`

## Guardrails

- Do not add remote-service assumptions.
- Do not add magical inference from test code.
- Do not add HTML before JSON and Markdown stay stable.
- Do not add impact analysis before the graph contract settles.
- Do not change ID grammar or node/edge kinds without an ADR.

## Proof surface

A good change usually updates one or more of:

- scenario tests in `core`
- fixture repo coverage
- golden JSON/Markdown output
- docs and ADRs if the public contract changes

## Commands

Use the `xtask` flow once the toolchain is available:

```bash
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full
cargo run -p xtask -- smoke
cargo run -p xtask -- docs-check
```

## Lane operating rules

- Before each lane PR:
  - run `git diff --name-status main...HEAD` and confirm the file set matches the PR title/body.
  - update the PR body with exact validation commands and their outcomes.
  - keep one proof obligation per PR.
- Preferred work order after foundation merge:
  1. closeout / release receipt
  2. dogfood scorecard
  3. metadata coverage expansion
  4. review-packet polish
  5. scope diagnostics refinement
  6. scaffold-from-gap hardening
  7. protocol compatibility pass
  8. agent handoff ergonomics
- Cleanup after PRs:
  - remove scratch/generated files created for experiments
  - clear stale cargo artifacts when they interfere with commands
  - leave `git status` clean
- Maintain explicit goal artifacts:
  - archive completed active-goal manifests under `.codex/goals/archive/`
  - keep `.codex/goals/active.toml` aligned to the current operational lane
