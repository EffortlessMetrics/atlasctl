---
atlas:
  id: adr:0001-product-scope-and-non-goals
  kind: adr
  title: Product scope and non-goals
  explains:
    - req:deterministic-atlas
---
# ADR 0001: Product scope and non-goals

`atlasctl` owns one job: compile a repo’s behavior and proof topology into a stable atlas.

It does not own:

- code intelligence
- test execution
- merge policy
- hosted reporting
