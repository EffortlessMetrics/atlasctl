# Agent Handoff: v0.1 Proof-Topology Lane

## Current lane state

- Foundation implementation for v0.1 proof topology is merged.
- Open implementation PRs are intentionally not in this repo lane (only docs/tooling overlap PRs remain separate).
- Current operational focus is closeout -> dogfood -> polish follow-up PRs.

## What to run when continuing this lane

From clean `main`:

```bash
git status --short
git diff --name-status main...HEAD
rtk cargo fmt --all -- --check
rtk cargo test --workspace
rtk cargo run -p xtask -- ci-full
rtk cargo run -p xtask -- docs-check
rtk cargo run -p xtask -- schema --check
rtk cargo run -p atlasctl-cli -- doctor --profile ci --repo-root .
rtk cargo run -p atlasctl-cli -- impacted --base main --head HEAD --format review-packet --repo-root .
rtk cargo run -p atlasctl-cli -- why --repo-root . --path crates/atlasctl-core/src/lib.rs
```

## Source-of-truth for this lane

- `.codex/goals/active.toml`
- `atlas/core.atlas.yaml`
- `docs/handoffs/v0.1-proof-topology-closeout.md`
- `docs/metadata-conventions.md`

## Next action

- Keep PR scope to one obligation and avoid broadening a review-ready PR.
- Preserve existing proof/router behavior; do not add CI routing logic here.
- Next natural lane PR after closeout handoff is `docs/` + `schema/golden` protocol compatibility cleanup if required by future evidence gaps.
