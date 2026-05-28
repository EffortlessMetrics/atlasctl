# atlasctl Real-Repo Dogfood Scorecard

## Target
- **Repo:** `H:\Code\Rust\atlasctl`
- **Diff:** `main..HEAD`
- **Commands:**
  - `atlasctl doctor --profile ci --repo-root .`
  - `atlasctl impacted --base main --head HEAD --format review-packet --repo-root .`
  - `atlasctl impacted --base main --head HEAD --format json --repo-root .`
  - `atlasctl why --path crates/atlasctl-cli/src/main.rs --format markdown --repo-root .`

## Execution Notes
- The JSON form is stored locally at `.tmp/atlasctl-dogfood.json` for tooling-only verification and is not committed.
- Command runs completed successfully without diagnostics from `doctor`.

## Results

| Metric | Value |
| --- | --- |
| Total changed paths | 32 |
| Covered by ownership/touches selectors | 4 |
| Uncovered changed paths | 28 |
| Uncovered rate | 87.5% |
| Impacted nodes in review packet | 16 |
| Missing-evidence diagnostics | 0 |
| Scope warning count | 3 |
| Suggested-fix entries | 3 |
| Why-query answer for touched file | returns deterministic proof chain with `scen:automated-scaffolding-reduces-friction` |

## Diagnostics observed
- Path-set warning: mixed docs + implementation change scope
- Path ownership warning: many files without selector coverage
- Protocol/schema warning: schema change not linked to protocol spec/proposal artifact metadata

## Recommended next action for repo hygiene
- Expand `owns`/`touches` coverage so generated support files, snapshots, and docs artifacts participate in the same source-of-truth model as behavior/proof/code.
- Link schema and protocol-protocol files to explicit roadmap/spec/ADR nodes as part of PR scoping.
- Keep PRs narrower in mixed doc/implementation scenarios so review packets stay deterministic and scoped.
