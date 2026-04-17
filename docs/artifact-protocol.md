# Artifact Protocol

This document defines the formal artifact protocol for `atlasctl`, ensuring stable, versioned, and trustworthy machine-facing outputs.

## Protocol Surfaces

`atlasctl` provides four primary machine-facing JSON surfaces:

1.  **Atlas Graph (`atlas.json`)**: The canonical representation of the repository proof topology.
2.  **Impact Analysis (`impact.json`)**: review-time mapping of diffs to behaviors.
3.  **Proof Chain (`why.json`)**: Curated semantic projection for a specific node or path.
4.  **Diagnostics (`doctor.json`)**: maintenance and drift reports.

## Versioning Policy

-   **Schema Version**: The `schema_version` field in `atlas.json` tracks material changes to the canonical graph model.
-   **Semantic Versioning**: `atlasctl` follows SemVer for its CLI and crate interfaces.
-   **Breaking Changes**: Material removals or renames of existing JSON fields are considered breaking changes and will trigger a major version bump and a schema version increment.

## Compatibility Rules

To maintain a stable control surface, `atlasctl` adheres to the following compatibility rules:

### 1. Stable Fields
-   Core identifiers (`id`, `kind`, `role`) are stable.
-   Provenance and location structures are stable.
-   Metrics keys are stable.

### 2. Canonical Ordering
-   All JSON lists (nodes, edges, diagnostics, impact hits) are deterministically sorted by ID or path to ensure stable diffs and reproducible artifacts.

### 3. Path Portability
-   All paths in artifacts are **repo-relative** and **forward-slash normalized**.
-   Absolute machine-local paths (e.g., `/home/user/...` or `C:\Users\...`) are strictly forbidden in machine-facing outputs.

## Release Gate: Schema Verification

The `xtask schema --check` task is a formal part of the release bar. It ensures:
1.  All committed schemas in `schemas/` match the current implementation.
2.  Any intentional protocol changes are explicitly reviewed alongside the code.

To update schemas after a valid protocol change:
```bash
cargo run -p xtask -- schema
```

## Future Expansion

The protocol is designed to be extensible. New fields may be added (non-breaking) to provide deeper code intelligence or better integration with remote services in future versions.
