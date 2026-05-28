---
atlas:
  id: support_tier:docs
  kind: support_tier
  title: Docs and README support tier
  summary: >
    Docs and README claims must remain linked to a proof command and governable policy
    surfaces so user-facing assertions are explicitly justified.
  proves:
    - cmd:docs-check
  claims:
    - claim:readme-doc-truth
  documents:
    - spec:proof-topology-stack
    - goal:ship-proof-topology-stack
    - closeout:v0-1-proof-topology
---

# Support Tiers

This file records the support layer for repository claims that are visible to users and
reviewers.

- Documentation claims are expected to be discoverable from source-of-truth metadata.
- Every claim must be linked to a proof command for deterministic verification.
