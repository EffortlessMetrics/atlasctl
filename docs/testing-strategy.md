---
atlas:
  id: guide:testing-strategy
  kind: guide
  title: Testing strategy
  documents:
    - req:deterministic-atlas
---
# Testing strategy

The repository treats verification as architecture.

## Core

- scenario tests
- property tests
- mutation later on critical semantics

## Discovery

- fixture repo coverage
- malformed metadata coverage
- frontmatter coverage

## Render

- golden `atlas.json`
- snapshot `atlas.md`

## CLI

- smoke tests
- exit-code tests
- query and trace output checks
