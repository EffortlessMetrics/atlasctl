---
atlas:
  id: adr:0002-canonical-atlas-graph-and-ids
  kind: adr
  title: Canonical atlas graph and IDs
  explains:
    - req:deterministic-atlas
---
# ADR 0002: Canonical atlas graph and IDs

Date: 2026-04-16
Status: Accepted

## Decision

The graph model is explicit and stable:

- strongly typed node kinds
- typed edge kinds
- explicit `owns` / `touches` path semantics
- stable IDs in `<kind>:<slug>` form
- role expectations by node kind (behavior/proof/document/artifact/command/infra)
- canonical output in `atlas.json`

## Why

- Deterministic IDs make diffs and diff-reviewing stable.
- Typed edges prevent accidental cross-domain assumptions.
- A canonical contract is required for automated consumers and CI evidence.

## Consequences

- New graph kinds and edge kinds require ADR updates.
- All output serializers must preserve canonical ordering.
