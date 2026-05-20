# Testing strategy

The repo’s verification model is operational proof, not synthetic test volume.

## Core contracts

- scenario tests for validation, trace semantics, why proof chains, and impact expansion
- property tests for determinism and integrity (normalization, absolute-path rejection, ordering)
- mutation tests for diff-critical paths in core paths
- owner/path coverage fixtures for ownership vs participation edge cases

## Discovery checks

- fixture-repo tests (valid/minimal, drift, broken links, duplicate ids)
- malformed fragment and frontmatter parsing tests
- CODEOWNERS overlay tests

## Render and schema checks

- golden snapshots for `atlas.json`, `atlas.md`, `impact`/`why` artifacts, and summary packets
- schema generation and compatibility checks:
  - `cargo run -p xtask -- schema`

## CLI and integration checks

- smoke and exit code tests
- `query`/`trace` output checks
- end-to-end `doctor`/`impacted`/`why` output tests
- `gh-summary` and `review-packet` projections

## Required verification commands

- `cargo run -p xtask -- ci-fast`
- `cargo run -p xtask -- ci-full`
- `cargo run -p xtask -- docs-check`
- `cargo run -p xtask -- schema --check`
- `cargo run -p xtask -- mutants`

## Strategic quality gates

- stable `doctor` results for the same working tree
- deterministic `impacted` and `why` outputs for unchanged input
- explicit handling of coverage gaps (no silent path blind spots)
