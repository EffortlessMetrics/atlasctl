# Coverage

Codecov coverage is Rust execution-surface evidence.

## What the Coverage Lane Answers

The Coverage workflow and Codecov dashboard answer:

> Did tests execute this Rust surface?

## What Coverage Does NOT Answer

Coverage does **not** prove:

- whether atlas graph construction is correct,
- whether requirement coverage is complete,
- whether scenarios are complete,
- whether fixtures model behavior correctly,
- whether command and artifact topology is correct,
- whether impact mapping is correct,
- whether `why`, `query`, or `trace` output is semantically complete,
- whether JSON schemas are compatible,
- whether golden snapshots are current,
- whether release readiness is proven.

Those are **separate proof lanes**.

## Workflow Details

The Coverage workflow runs on:

- **push to `main`**: Generates coverage and uploads to Codecov (blocking on token presence)
- **`workflow_dispatch`**: Manual trigger for ad-hoc coverage runs
- **PR labels**: Runs on PRs labeled `coverage`, `full-ci`, or `ci:full`

## Artifacts

Durable receipts from each coverage run:

- `coverage.json` — machine-readable coverage data
- `coverage.txt` — human-readable summary
- `lcov.info` — Codecov-compatible LCOV format
- GitHub Actions coverage artifact (14-day retention)
- Codecov dashboard (permanent, if Codecov token is configured)

## Configuration

Coverage behavior is controlled by:

- `.github/workflows/coverage.yml` — GitHub Actions workflow
- `codecov.yml` — Codecov status and reporting configuration
- `CODECOV_TOKEN` secret (optional; coverage still generates artifacts without it)

## Codecov Comments

Codecov comments are **disabled** (`comment: false` in `codecov.yml`).

Coverage status is **advisory** (informational; non-blocking on PRs).

## Interpreting Coverage Dips

A coverage dip does not necessarily mean:

- Tests are broken (tests may be skipped, platform-specific, or gated)
- Implementation is incomplete (code may be correct but hard to reach in tests)
- The feature is untested (it may be tested at a higher level or by manual verification)

It **does** mean:

- Some code path executed in prior runs is not being reached now
- Investigation is warranted to understand the change

## Adding Coverage to PRs

To trigger coverage on a PR without waiting for `main`:

1. Label the PR with `coverage`, `full-ci`, or `ci:full`
2. Coverage will run on the next push to the PR branch

## Viewing Coverage Results

- **Codecov dashboard**: https://codecov.io/gh/EffortlessMetrics/atlasctl
- **GitHub Actions**: https://github.com/EffortlessMetrics/atlasctl/actions/workflows/coverage.yml
- **PR artifacts**: Download `coverage-report` from the PR's Actions tab (if generated)
