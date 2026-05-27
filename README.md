# atlasctl

[![CI](https://github.com/EffortlessMetrics/atlasctl/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/EffortlessMetrics/atlasctl/actions/workflows/ci.yml)
[![Coverage](https://github.com/EffortlessMetrics/atlasctl/actions/workflows/coverage.yml/badge.svg?branch=main)](https://github.com/EffortlessMetrics/atlasctl/actions/workflows/coverage.yml)
[![Codecov](https://codecov.io/gh/EffortlessMetrics/atlasctl/branch/main/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/atlasctl)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`atlasctl` is a local-first scenario and proof atlas for Rust-style repositories.

It transforms repository metadata into a deterministic, queryable graph to provide **behavior-aware review and proof routing**.

## Core Operational Workflow

`atlasctl` is designed to be part of your daily development and CI workflow.

### 1. Self-Policing with `doctor`
Catch graph drift, dead selectors, and orphan nodes before they become a problem.
```bash
atlasctl doctor
```

### 2. Review-time Impact Analysis
Map a diff to behavior, proof surfaces, and documentation.
```bash
# Analyze impact of local changes vs main
atlasctl impacted --base main --head HEAD

# Use in CI for a compact summary
atlasctl impacted --format gh-summary

# Get a review packet for a PR-style path diff
atlasctl review-packet --base main --head HEAD

# Or for explicit path-focused review packets
atlasctl review-packet --paths "crates/engine" "src/main.rs"
```

### 3. Semantic Navigation with `why`
Project a short, readable proof chain for any node or path to understand its purpose and verification.
```bash
atlasctl why --path crates/atlasctl-core/src/lib.rs
```

## Onboarding and Scaffolding

Bootstrap a new atlas or add new proof surfaces quickly:

```bash
# Initialize a new repository
atlasctl init

# Scaffold a new scenario, artifact, or requirement
atlasctl scaffold scenario my-new-feature
```

## Repository Intelligence

`atlasctl` compiles declared metadata about:
- **Requirements**: Behavioral goals of the system.
- **ADRs**: Architecture Decision Records.
- **Scenarios**: Concrete behaviors that prove requirements.
- **Fixtures**: Models or data used in proof.
- **Commands**: Operational tasks or test runners.
- **Artifacts**: Outputs to be inspected.
- **Crates**: Infrastructure components.

The result is a stable `atlas.json` (machine-facing) and a human-readable `atlas.md`.

## Installation

```bash
cargo build --release
```
Requires Rust 1.92 or later (Edition 2024).

## Current Status (v0.1.0)

- ✅ **Operational CLI**: Deep support for `doctor`, `impacted`, and `why`.
- ✅ **Deterministic Artifacts**: Stable JSON schema and repo-relative pathing.
- ✅ **Owner Overlays**: Integrated `CODEOWNERS` support.
- ✅ **CI Optimized**: Dedicated GitHub step summary output.
- ✅ **Scaffolding**: Low-friction `init` and `scaffold` commands.

## License

MIT License - see [LICENSE](LICENSE) file for details.
