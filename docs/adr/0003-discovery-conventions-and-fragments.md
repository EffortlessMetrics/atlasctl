---
atlas:
  id: adr:0003-discovery-conventions-and-fragments
  kind: adr
  title: Discovery conventions and fragments
  explains:
    - req:queryable-proof-topology
---
# ADR 0003: Discovery conventions and fragments

Date: 2026-04-16  
Status: Accepted

## Decision

Discovery is explicit-first and adapter-based:

- `atlas.toml` controls repository-level settings.
- `*.atlas.yaml` supplies graph fragments.
- Markdown frontmatter supplies lightweight nodes, scenario links, and docs references.
- Cargo workspace metadata identifies Rust crates.
- path declarations split ownership (`owns`) and participation (`touches`).

## Evidence

- All metadata sources are merged into a single discovery batch.
- Duplicate IDs and malformed references are validated before graph projection.

## Why

Explicit metadata reduces false positives and makes repository intent inspectable
without requiring custom heuristics.

## Consequences

- Markdown without frontmatter remains discoverable as plain text but is not graph-typed.
- Fragments must keep IDs stable and unique across directories.
