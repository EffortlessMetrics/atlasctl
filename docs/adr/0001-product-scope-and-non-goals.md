---
atlas:
  id: adr:0001-product-scope-and-non-goals
  kind: adr
  title: Product scope and non-goals
  explains:
    - req:deterministic-atlas
---
# ADR 0001: Product scope and non-goals

Date: 2026-04-16
Status: Accepted

## Decision

`atlasctl` is a local compiler and navigator for behavior/proof metadata.
It does not perform remote inference, static analysis of code bodies beyond
declared metadata, or CI policy decisions.

## Scope

- `atlasctl` compiles explicit repository behavior/proof metadata into deterministic artifacts.
- It is used operationally through `impacted`, `doctor`, and `why`.
- `query` and `trace` provide navigation, but do not replace review workflows.
- It exports machine contracts for review, CI, and local projection.

## Non-goals

- code intelligence (no language semantic inference)
- test execution orchestration (it reads test status, it does not run tests)
- merge policy and auto-enforcement
- hosted reporting or remote control plane
- probabilistic/AI inference for proof claims

## Outcome

The product stays explicit, local-first, and suitable for review-native contracts.
