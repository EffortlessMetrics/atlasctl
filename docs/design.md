---
atlas:
  id: guide:design-philosophy
  kind: guide
  title: Design philosophy
  documents:
    - req:deterministic-atlas
---
# Design philosophy

`atlasctl` is a review-time compiler for explicit repository proof topology.

## Principles

- Prefer explicit metadata over inferred conventions.
- Keep parsing and filesystem concerns in adapters.
- Keep graph model, assembly, validation, query, trace, and impact in the core.
- Keep CLI orchestration and transport at the outer edge.
- Keep output rendering deterministic and schema-driven.
- Make `impacted`, `doctor`, and `why` the primary workflows.

## Boundary split

1. **Core domain (`atlasctl-core`)**: graph model, assembly, validation, query, trace.
2. **Discovery adapters (`atlasctl-discover-fs`)**: file scanning, frontmatter parsing, workspace metadata.
3. **App surface (`atlasctl-app`)**: use-case composition, command orchestration, and validation policy.
4. **Render (`atlasctl-render`)**: format conversion for JSON, markdown, gh-summary, and review packet.
5. **CLI (`atlasctl-cli`)**: command parsing, argument validation, and command output formatting.

## Determinism contract

The design contract for every command path is:

1. normalize inputs,
2. resolve into typed graph objects,
3. validate and classify diagnostics,
4. render deterministically into the requested projection.

No command path is allowed to mutate output ordering or ordering-sensitive structures
after render time.

## Why command hierarchy exists

`query` and `trace` are navigation primitives.

The operational contract is:

1. `impacted`: what changed and where review attention is required.
2. `doctor`: graph health and policy conformance.
3. `why`: short proof chain for IDs and paths.
