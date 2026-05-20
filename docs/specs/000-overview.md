# 000 - Overview

`atlasctl` is a **local-first proof-topology control surface for repositories**.

It compiles explicit repository metadata into a deterministic graph so maintainers can answer:

- What behavior exists?
- What proves it?
- What changed?
- Who owns it?
- What evidence is missing?

## What `atlasctl` is not

| Not this | Why not |
|---|---|
| Deep language-specific code intelligence | It stays repo-level and review-level, not file-level inference |
| Test runner | It points to proof commands; it does not own execution |
| Hosted dashboard | The differentiator is local-first, not remote hosting |
| AI behavior inference | The model is explicit, reviewable metadata |
| Generic docs renderer | Markdown is a projection, not the product |
| Workflow platform | It feeds workflows; it should not own them |

## Core daily workflow

The front door is review-time proof operations:

1. `impacted` — what changed, and what should reviewers inspect?
2. `doctor` — is the repo proof map healthy?
3. `why` — for a node/path, what is the short proof chain?

Supporting commands (`build`, `check`, `query`, `trace`, `export`, `init`, `scaffold`) are useful, but they do not define the core question the tool answers.

## Trust boundary

- `atlasctl` ingests explicit metadata plus deterministic discovery inputs.
- It does not infer behavior from arbitrary code beyond declared proof assertions.
- All outputs are explicit artifacts with stable schema and ordering.

## Product definition

`atlasctl` is a deterministic compiler for repository behavior/proof metadata, with review-time and CI-native projections.
