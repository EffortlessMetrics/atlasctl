# 040 - Markdown Frontmatter

Markdown metadata is intentionally light. Do not encode large graph objects in documents.

Frontmatter should define identity plus relationships, not business logic.

## Required fields

```yaml
---
atlas:
  id: adr:0001-product-scope-and-non-goals
  kind: adr
  role: document
  title: Product scope and non-goals
---
```

## Optional fields

- `summary`
- `role`
- `owns` / `touches`
- `explains`
- `proves`
- `documents`
- `paths` (legacy alias to `owns`)
- `attrs`

## Path fields

- `owns`: ownership declarations.
- `touches`: participation declarations.

Frontmatter remains explicit and reviewable; if a document needs deeper structure, move it to `*.atlas.yaml`.
