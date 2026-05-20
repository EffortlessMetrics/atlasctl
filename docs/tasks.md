---
atlas:
  id: guide:roadmap
  kind: guide
  title: Roadmap
  documents:
    - req:deterministic-atlas
---
# Roadmap

This roadmap is source-of-truth for sequencing. The repository should prioritize review-time value and contract durability.

## Completed

- ✅ Core operational command set (`doctor`, `impacted`, `why`) and deterministic artifact output.
- ✅ Stable graph/diagnostic foundations in `atlasctl-core` and discovery in `atlasctl-discover-fs`.
- ✅ Golden-based projection stability for command outputs.
- ✅ Self-dogfooding loop via internal atlas metadata and CLI command coverage.
- ✅ Microcrate consolidation prep in workspace shape (`atlasctl-codes` and `atlasctl-ports` already removed).

## In progress

- 🟡 `doctor`/`impacted`/`why` contract depth and stability.
- 🟡 Artifact protocol hardening (`doctor.json`, `impact.json`, `why.json`, summary/protocol packets).
- 🟡 Real-repo convergence and gap-metrics runbook.

## Phase 1 — Converge and simplify

1. **Align public docs with product intent**
   - README/front-door sequence: `impacted` → `doctor` → `why`.
   - Keep micro-level docs consistent with `docs/specs/*`.
   - ✅ Owner: docs.

2. **Source-of-truth graph and proof protocol**
   - Confirm `docs/specs` captures required behavior, validation, and artifact contracts.
   - Add clear links from `docs/architecture.md`, `docs/design.md`, and `docs/artifact-protocol.md`.
   - ✅ Owner: docs.

3. **Ownership semantics (owns vs touches)**
   - Keep the distinction explicit in docs.
   - `owns` remains exclusive per path.
   - `touches` overlap is allowed.
   - ✅ Owner: core semantics / design.

## Phase 2 — Freeze graph semantics

1. **Add role to nodes**
   - Enforce explicit `role` in fragment and frontmatter contracts.
   - Keep ownership semantics (`owns` vs `touches`) deterministic.

2. **Fixture coverage**
   - overlapping participation
   - duplicate ownership
   - unowned path
   - requirement without proof
   - command without artifact
   - doc-only node
   - ✅ Outcome: deterministic diagnostics and profile behavior.

3. **Role clarity**
   - Document role expectations for command-proofing and infra nodes.
   - Ensure docs, tests, and diagnostics present role-consistent language.
   - ✅ Owner: docs + core.

## Phase 3 — Harden artifact protocol

1. Finalize JSON schemas and compatibility rules.
2. Add/keep golden tests for:
   - atlas.json
   - why
   - impacted
   - doctor
   - gh-summary
   - review-packet
3. Lock absolute-path rejection and slash normalization.

## Phase 4 — Make operational commands excellent

1. Improve `doctor` evidence depth and gap reporting.
2. Improve `impacted` path→owner mapping and gap classification.
3. Improve `why` proof-chain brevity and explainability.
4. Add command integration snapshots for these top 3 workflows.

## Phase 5 — Convergence and deployment readiness

1. Real-repo proof pass on 3–5 repos.
2. Track these metrics:
   - uncovered changed-path rate
   - duplicate ownership rate
   - ambiguity rate
3. Use metrics to tune defaults and profile severity.

## Definition of done

An item is complete when:

- associated specs are updated,
- validation is green (`cargo run -p xtask -- ci-full` or equivalent),
- reviewer-facing proof is clear in `doctor`/`impacted`/`why`,
- and release-impacting schema changes are noted in `docs/specs/090-compatibility.md`.

## Future structural work

The following consolidation remains scoped to later phases:

- `atlasctl-fixtures` to support crate role only (or demote to tests/support), unless proven stable as a long-lived crate.
