---
atlas:
  id: adr:0005-verification-matrix
  kind: adr
  title: Verification matrix
  explains:
    - req:deterministic-atlas
---
# ADR 0005: Verification matrix

Date: 2026-04-16
Status: Accepted

## Decision

Verification is staged by scope:

- **Type-level checks**: unit, schema, and invariants.
- **Core behavior checks**: scenario tests for trace/impact/validation.
- **Discovery checks**: repository fixtures and malformed-metadata tests.
- **Render checks**: golden and snapshot stability.
- **CLI checks**: end-to-end command behavior and exit status.
- **Mutation checks**: `atlasctl-core` critical paths.
- **Convergence checks**: real-repo coverage and gap telemetry.

## Why

Layered checks allow faster developer feedback and stronger release confidence without
blurring local development with full release gates.

## Consequences

- `ci-fast`: local and pre-merge confidence.
- `docs-check` gates graph health and metadata correctness.
- `ci-full`: includes smoke and artifact validation.
