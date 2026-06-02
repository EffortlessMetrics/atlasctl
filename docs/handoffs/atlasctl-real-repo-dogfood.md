# atlasctl Real-Repo Dogfood Scorecard

## Scope

This is the first scorecard for `atlasctl` itself as a real-repo consumer of atlasctl metadata. It does not use fixture repos.

- **Repo:** `H:\Code\Rust\atlasctl`
- **Command surface under review:** `doctor`, `impacted`, `why`

## Method

For each sampled historical PR, run:

```bash
rtk cargo run -p atlasctl-cli -- doctor --repo-root . --profile ci
rtk cargo run -p atlasctl-cli -- impacted --repo-root . --base <base> --head <head> --format review-packet
rtk cargo run -p atlasctl-cli -- impacted --repo-root . --base <base> --head <head> --format json
rtk cargo run -p atlasctl-cli -- why --repo-root . --path crates/atlasctl-core/src/lib.rs
```

Then derive:
- changed path count
- uncovered count / uncovered rate
- impacted node count
- missing evidence count
- scope warning count
- response latency for command set (time-to-answer)

## Sample Set (historical real PR ranges)

| Sample | Base..Head | Changed Paths | Covered | Uncovered | Uncovered Rate | Impacted Nodes | Missing Evidence | Scope Warnings |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| PR #21 foundation | `83d1049..12ea4da` | 39 | 39 | 0 | 0.0% | 31 | 0 | 1 |
| PR #23 policy+discovery | `8e64863..c933cbe` | 4 | 4 | 0 | 0.0% | 26 | 0 | 0 |
| PR #24 dogfood-surface | `c933cbe..7f0561c` | 1 | 1 | 0 | 0.0% | 10 | 0 | 0 |
| Local follow-up (`main`) | `9ba6430..5d44572` | 6 | 6 | 0 | 0.0% | 14 | 0 | 1 |
| External sample: shiplog PR #150 | `e0f5c7c..7111ada` | 9 | 0 | 9 | 100% | 0 | 0 | 3 |

## Real-repo portability check (external reference)

To test portability, we ran the same command set on `H:\Code\Rust\tokmd`
at commit `95a5034abeeb1552ce10432647a19bff2b98b08a` against base
`4b431559ee2cf3eda81b499c1cf678d9ab2fde6a`:

- `rtk cargo run -p atlasctl-cli -- doctor --repo-root H:\Code\Rust\tokmd --profile ci`
- `rtk cargo run -p atlasctl-cli -- check --repo-root H:\Code\Rust\tokmd --profile ci`
- `rtk cargo run -p atlasctl-cli -- impacted --repo-root H:\Code\Rust\tokmd --base 4b431559ee2cf3eda81b499c1cf678d9ab2fde6a --head 95a5034abeeb1552ce10432647a19bff2b98b08a --format review-packet`
- `rtk cargo run -p atlasctl-cli -- impacted --repo-root H:\Code\Rust\tokmd --base 4b431559ee2cf3eda81b499c1cf678d9ab2fde6a --head 95a5034abeeb1552ce10432647a19bff2b98b08a --format json`
- `rtk cargo run -p atlasctl-cli -- why --repo-root H:\Code\Rust\tokmd --path crates/tokmd-core/src/lib.rs`

Observed behavior:

- `doctor` and `check` both parse crates successfully and return `status: ok` with
  `58` structural warnings (`uncovered_crate` for each discovered crate).
- The repo-level impact output for a 1-file commit had:
  - Changed paths: `1`
  - Uncovered: `1`
  - Uncovered rate: `100%`
  - Impacted nodes: `0`
  - Missing evidence: `0`
  - Scope warnings: `1` (`1` changed paths are not covered by any known ownership/touches selector)
- `review-packet` for this sample reports exactly that the changed path is uncovered and suggests adding `owns`/`touches` coverage.
- `why` on a source file returns `No matching node found`, because no source-of-truth metadata is yet defined for tokmd.

Additional external sample from `H:\Code\Rust\shiplog` (PR #150, `e0f5c7c..7111ada`):

- `rtk cargo run -p atlasctl-cli -- doctor --repo-root H:\Code\Rust\shiplog --profile ci`
- `rtk cargo run -p atlasctl-cli -- check --repo-root H:\Code\Rust\shiplog --profile ci`
- `rtk cargo run -p atlasctl-cli -- impacted --repo-root H:\Code\Rust\shiplog --base e0f5c7c --head 7111ada --format review-packet`
- `rtk cargo run -p atlasctl-cli -- impacted --repo-root H:\Code\Rust\shiplog --base e0f5c7c --head 7111ada --format json`
- `rtk cargo run -p atlasctl-cli -- why --repo-root H:\Code\Rust\shiplog --path xtask/src/tasks/check_support_tiers.rs`

Observed behavior:

- `doctor` and `check` currently fail with `26` diagnostics (`1` error, `25` warnings). The error is `active_goal_missing_plan`; the dominant warnings are `policy_file_legacy_no_atlas` for policy files without atlas sections plus active-goal incompleteness warnings.
- `review-packet` reported `9` changed paths, `9` uncovered (`100%` coverage), `0` impacted nodes, and `3` scope warnings.
- `why` on a changed source file returns `No matching node found`, confirming no proof-chain coverage without source-of-truth metadata.

## What the scorecard shows

- On atlasctl PR history, uncovered rate is currently `0%` for the three sampled PRs after `scen:full-release-verification` expansion from PR #24.
- One known scope warning remains for PR #21: mixed docs + implementation surface.
- No missing-evidence diagnostics in any sampled range.
- `review-packet` consistently produced deterministic changed-path coverage and next-action hints for all samples.

## Real-repo command latency (sample: current `main`)

- `doctor --profile ci`: **~3.45s**
- `impacted --format review-packet`: **~3.93s**
- `why --path crates/atlasctl-core/src/lib.rs`: **~3.47s**

Command runs remained stable and produced consistent output for this repo as evidence carrier.

## Residual gaps for lane continuation

- External repositories without source-of-truth metadata can still be analyzed for discovery-only structural warnings, but no proof-trace (`why`) output is possible.
- Legacy `policy/*.toml` repositories without Atlas metadata sections now emit non-blocking `policy_file_legacy_no_atlas` warnings and are skipped from atlas discovery, improving review-packet reach while still signaling migration work.
- This indicates portability work for broader adoption: discoverability scales, but `proof` surfaces are empty without metadata in `atlas.toml` + metadata files.
- **Ambiguous selector rate** and **maintainer correction rate** are still not directly emitted by current `impacted` JSON; these need either a CLI metric extension or scorecard-side script.
- Additional external-repo samples are still pending to measure portability variance across repo shapes.

## Suggested follow-up

- Keep PR scope discipline from this sample; mixed docs/implementation PRs can trigger actionable scope warnings and weaken review clarity.
- Expand scorecard tooling in a follow-up PR to emit and persist these additional metrics automatically.
- For external-adoption support, add a clear onboarding path in docs for defining minimal coverage metadata so repos receive non-empty proof surfaces instead of structural-only output.
