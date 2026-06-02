# atlasctl Real-Repo Dogfood Scorecard

## Scope

This is the first scorecard for `atlasctl` itself as a real-repo consumer of atlasctl metadata. It does not use fixture repos.

- **Repo:** `H:\Code\Rust\atlasctl`
- **Command surface under review:** `doctor`, `impacted`, `why`

## Method

For each sampled historical PR, run:

```bash
cargo run -p atlasctl-cli -- doctor --repo-root . --profile ci
cargo run -p atlasctl-cli -- impacted --repo-root . --base <base> --head <head> --format review-packet
cargo run -p atlasctl-cli -- impacted --repo-root . --base <base> --head <head> --format json
cargo run -p atlasctl-cli -- why --repo-root . --path crates/atlasctl-core/src/lib.rs
```

Then derive:
- changed path count
- uncovered count / uncovered rate
- impacted node count
- missing evidence count
- scope warning count
- response latency for command set (time-to-answer)

## Sample Set (historical real PR ranges)

| Sample | Base..Head | Changed Paths | Covered? | Uncovered | Uncovered Rate | Impacted Nodes | Missing Evidence | Scope Warnings |
|---|---|---:|---:|---:|---:|---:|---:|
| PR #21 foundation | `83d1049..12ea4da` | 39 | 39 | 0 | 0.0% | 31 | 0 | 1 |
| PR #23 policy+discovery | `8e64863..c933cbe` | 4 | 4 | 0 | 0.0% | 26 | 0 | 0 |
| PR #24 dogfood-surface | `c933cbe..7f0561c` | 1 | 1 | 0 | 0.0% | 10 | 0 | 0 |

## What the scorecard shows

- Zero uncovered paths in all three sampled PR ranges after `scen:full-release-verification` surface expansion from PR #24.
- One known scope warning remains for PR #21: mixed docs + implementation surface.
- No missing-evidence diagnostics in any sampled range.
- `review-packet` consistently produced executable proof command sets and deterministic impacted truth surface sections for all samples.

## Real-repo command latency (sample: current `main`)

- `doctor --profile ci`: **~3.45s**
- `impacted --format review-packet`: **~3.93s**
- `why --path crates/atlasctl-core/src/lib.rs`: **~3.47s**

Command runs remained stable and produced consistent output for this repo as evidence carrier.

## Residual gaps for lane continuation

- **Ambiguous selector rate** and **maintainer correction rate** are still not directly emitted by current `impacted` JSON; these need either a CLI metric extension or scorecard-side script.
- External-repo dogfood is still pending: this scorecard is a required first milestone, with the next step being one additional repo for portability.

## Suggested follow-up

- Keep PR scope discipline from this sample; mixed docs/implementation PRs can trigger actionable scope warnings and weaken review clarity.
- Expand scorecard tooling in a follow-up PR to emit and persist these additional metrics automatically.
