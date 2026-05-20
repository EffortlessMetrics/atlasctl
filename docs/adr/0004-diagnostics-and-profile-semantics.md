---
atlas:
  id: adr:0004-diagnostics-and-profile-semantics
  kind: adr
  title: Diagnostics and profile semantics
  explains:
    - req:deterministic-atlas
---
# ADR 0004: Diagnostics and profile semantics

Date: 2026-04-16  
Status: Accepted

## Decision

Validation is enforced through three profiles:

- `default`: local developer defaults.
- `ci`: stricter requirements for pre-merge checks.
- `strict`: release-grade quality with warnings-as-errors.

Diagnostics are strongly typed and deterministic.
Ownership conflict and coverage profiles are part of this contract.

## Why

- Progressive adoption is needed for legacy repositories.
- CI needs stronger guarantees than local defaults.
- Release-grade checks must expose warning debt before publishing.

## Consequences

- New rule families must define their default profile behavior.
- Consumers can choose profile by command argument without changing sources.
