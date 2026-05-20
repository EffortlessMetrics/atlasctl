# 090 - Compatibility

Artifact contracts should evolve slowly and intentionally.

## Compatibility rules

- Adding optional fields: **non-breaking**
- Removing fields: **breaking**
- Renaming fields: **breaking**
- Changing path semantics: **breaking**
- Relaxing validation semantics: **breaking when it weakens guarantees**
- Tightening defaults: **breaking** when consumers rely on old behavior

## Protocol evolution process

Before any breaking change:

1. Open/update ADR.
2. Regenerate schemas (`cargo run -p xtask -- schema`).
3. Update golden outputs and docs.
4. Run full CI checks and gate review.
5. Require release-note update and compatibility note.

## Migration discipline

- Keep explicit changelog entries for protocol behavior.
- Track compatibility decisions and schema changes in `docs/specs/090-compatibility.md` and release notes.
- Prefer additive fields and downgradable consumers.

## Compatibility-sensitive changes

- Diagnostic severity changes are breaking for CI consumers.
- Schema migration and golden updates are required for output shape changes.
