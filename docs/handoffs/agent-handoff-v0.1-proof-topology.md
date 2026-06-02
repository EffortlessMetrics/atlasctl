# Agent Handoff: v0.1 Proof-Topology Lane

## Current lane state

- Foundation implementation for v0.1 proof topology is merged.
- Open implementation PRs are intentionally not in this repo lane (only docs/tooling overlap PRs remain separate).
- Foundation closeout PRs are done; current focus is post-closeout review-router readiness.
- `.codex/goals/active.toml` now points to `goal:advance-review-packet-router` and `plan:post-closeout-router-readiness-v0-2`.
- This lane has been archived into `.codex/goals/archive/ship-proof-topology-stack-v0-1.toml`.
- Open PR hygiene audit (live):
  - `#2` `Enable Factory Droid automated code review` (head `add-factory-workflows-20260504024339`, unrelated infra lane).

## Open PR hygiene check

From this lane's perspective, the only open PR in this repository is #2 and it is outside the source-of-truth stack source work.

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
- Next natural lane PR after this handoff is post-closeout router-readiness work:
  - scope warning hardening,
  - review packet polish,
  - protocol compatibility cleanup if required by evidence gaps.
- New active-goal manifest handoff expectations:
  - Keep `.codex/goals/active.toml` as the current operational surface.
  - Add a new archived manifest for each completed lane in `.codex/goals/archive/`.
  - Record next planned surface (claims, policy, scope, review packet, protocol) in PR title/body before opening.
- Keep workspace hygiene between PRs:
  - close stale local branches/worktrees before handoff,
  - remove scratch/golden-generation artifacts,
  - run `git diff --name-status main...HEAD` before each PR,
  - keep `git status` clean at handoff points.
