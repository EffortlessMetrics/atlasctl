# 020 - Config

`atlas.toml` controls discovery and validation policy, while fragments and frontmatter carry most metadata graph content.

## Canonical config shape

```toml
schema_version = 1

[repo]
id = "atlasctl"
name = "atlasctl"
default_branch = "main"

[discovery]
fragments = ["atlas/**/*.atlas.yaml"]
markdown = ["README.md", "docs/**/*.md"]
cargo = true
codeowners = true
ignore = [".git/**", "target/**", ".atlas/**"]
fragments_allow_hidden = false

[outputs]
dir = ".atlas"
path_style = "repo-relative-slash"
write_json = true
write_markdown = true

[profiles.default]
broken_refs = "error"
invalid_ids = "error"
duplicate_ids = "error"
dead_selectors = "warn"
orphan_nodes = "warn"
duplicate_owns = "error"
duplicate_touches = "warn"
absolute_paths = "error"
unproven_requirements = "warn"
uncovered_changed_paths = "warn"
schema_drift = "warn"

[profiles.ci]
extends = "default"
dead_selectors = "error"
orphan_nodes = "warn"
schema_drift = "error"
uncovered_changed_paths = "error"

[profiles.strict]
extends = "ci"
warnings_as_errors = true
require_roles = true
require_command_for_proof = true
require_requirement_proof = true
```

## Current supported config

The implementation currently supports:

- `[discovery].roots`
- `[discovery].ignore`
- `[profiles.{default,ci,strict}]` booleans:
  - `require_scenario_command`
  - `require_scenario_crate`
  - `require_artifact_producer`
  - `warnings_as_errors` (strict only today)

Planned support (canonical shape above) includes:

- `fragments_allow_hidden`
- `duplicate_touches`
- `absolute_paths`
- `schema_drift`
- `require_roles`
- `require_requirement_proof`
- `require_command_for_proof`

This is intentionally minimal and will be expanded to the canonical config in line with `030` and `090`.

## Config principles

- Explicit over inferred.
- Small and boring defaults.
- Profiles are stable names with predictable rule escalation.
- Parsing and validation are deterministic.
- Config changes should be reflected in `docs/specs/020-config.md` before implementation.
