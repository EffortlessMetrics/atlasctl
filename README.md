# atlasctl

`atlasctl` is a **local-first proof-topology control surface** for maintainers and reviewers.

It compiles explicit repository metadata into a deterministic graph so you can answer:

- What behavior exists?
- What proves it?
- What changed?
- Who owns it?
- What proof is missing?

## What atlasctl is

- Compile canonical artifacts with stable, repo-relative output (`.atlas/atlas.json` + projections).
- Answer proof questions (`why`), impact questions (`impacted`), and graph health (`doctor`) quickly during review.
- Track adoption through explicit metadata, not inference.
- Keep evidence local-first and review-oriented.

## What atlasctl is not

- A deep language intelligence engine.
- A test runner.
- A hosted decision platform.
- AI inference or probabilistic explanation layer.
- A generic docs renderer.

## Core operational workflow

1. **Map a diff to review targets**

```bash
atlasctl impacted --base main --head HEAD
```

2. **Check graph health**

```bash
atlasctl doctor --format json

atlasctl doctor --format gh-summary
```

3. **Explain an ID or file path**

```bash
atlasctl why req:deterministic-atlas
atlasctl why --path crates/atlasctl-core/src/lib.rs
```

4. **How is the graph built and projected?**

```bash
atlasctl build
atlasctl check --profile ci
atlasctl query scen:build-emits-canonical-atlas
atlasctl trace req:deterministic-atlas --direction reverse --depth 2
atlasctl export --format gh-summary --out .atlas/atlas.gh-summary.md
atlasctl export --format review-packet --out .atlas/atlas.review-packet.md
```

5. **Keep metadata friction low**

```bash
atlasctl init
atlasctl scaffold requirement deterministic-atlas
atlasctl scaffold scenario build-emits-canonical-atlas
atlasctl scaffold artifact atlas-json
```

## Command surface

- `init` — generate initial `atlas.toml`.
- `build` — compile graph and render artifacts.
- `check` — validate current graph for chosen profile.
- `doctor` — profile-driven health summary.
- `impacted` — review diff/path impact.
- `why` — short proof chain for ID or path.
- `query` / `trace` — graph exploration utilities.
- `export` — deterministic projection writer.
- `scaffold` — generate metadata stubs.

## Configuration and specs

- [Overview](docs/specs/000-overview.md): definition, trust boundaries, and workflow
- [Graph model](docs/specs/010-graph-model.md): kinds, roles, IDs, and edges
- [Config](docs/specs/020-config.md): discovery and profile contract
- [Metadata fragments](docs/specs/030-metadata-fragments.md): `*.atlas.yaml` shape
- [Markdown frontmatter](docs/specs/040-markdown-frontmatter.md): lightweight inline metadata
- [Validation profiles](docs/specs/050-validation-profiles.md): rule severity and policy
- [Artifact protocol](docs/specs/060-artifact-protocol.md): machine output contracts
- [Review impact](docs/specs/070-review-impact.md): diff-to-proof projection rules
- [CLI contract](docs/specs/080-cli-contract.md): command and format matrix
- [Compatibility](docs/specs/090-compatibility.md): schema and protocol evolution

## Install and build

```bash
cargo build --release
```

Requires Rust 1.92 or later (Edition 2024).

## Current status (v0.1.0)

- ✅ operational review flow (`doctor`, `impacted`, `why`)
- ✅ deterministic artifacts (`atlas.json`) and stable ordering
- ✅ ownership/participation metadata supported via `owns` and `touches`
- ✅ ownership diagnostics enforce duplicate-`owns` policy and track `touches` overlap
- ✅ `init` and `scaffold` are available for low-friction onboarding

## License

MIT — see [LICENSE](LICENSE).
