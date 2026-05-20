---
atlas:
  id: guide:requirements-and-specs
  kind: guide
  title: Requirements and specification map
  documents:
    - req:deterministic-atlas
---
# Requirements and specification map

This repository’s product requirements are defined as a specs corpus in `docs/specs`.

- [000-overview](docs/specs/000-overview.md)
- [010-graph-model](docs/specs/010-graph-model.md)
- [020-config](docs/specs/020-config.md)
- [030-metadata-fragments](docs/specs/030-metadata-fragments.md)
- [040-markdown-frontmatter](docs/specs/040-markdown-frontmatter.md)
- [050-validation-profiles](docs/specs/050-validation-profiles.md)
- [060-artifact-protocol](docs/specs/060-artifact-protocol.md)
- [070-review-impact](docs/specs/070-review-impact.md)
- [080-cli-contract](docs/specs/080-cli-contract.md)
- [090-compatibility](docs/specs/090-compatibility.md)

## Primary requirements snapshot

- **deterministic atlas compilation** from explicit metadata
- **queryable proof topology** for ownership, scenarios, and evidence
- **operational review workflows** centered on `impacted`, `doctor`, and `why`
- **low-friction onboarding** through `init` and `scaffold`
- **artifact contract stability** with schema-backed machine outputs

## Evidence obligations

- Every behavior requirement must map to one or more proving scenarios.
- Every scenario must be connected to commands/artifacts where applicable.
- Every changed path must produce stable impact classification or an explicit gap.
