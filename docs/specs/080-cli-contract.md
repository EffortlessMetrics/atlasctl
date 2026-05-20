# 080 - CLI Contract

Stable commands:

- `init`
- `build`
- `check`
- `doctor`
- `impacted`
- `why`
- `query`
- `trace`
- `export`
- `scaffold`

## Primary operational order

1. `impacted`
2. `doctor`
3. `why`
4. `query` / `trace`
5. `build` / `check` / `export` for persistence

## Formats by command

- `build`: `json`, `markdown`
- `check`: `text`, `json`, `markdown`, `gh-summary`
- `doctor`: `text`, `json`, `markdown`, `gh-summary`, `review-packet`
- `impacted`: `text`, `json`, `markdown`, `gh-summary`, `review-packet`
- `why`: `text`, `json`, `markdown`, `gh-summary`, `review-packet`
- `query` / `trace`: `text`
- `export`: `json`, `markdown`, `gh-summary`, `review-packet`

## Stability notes

- Output ordering and casing are stable.
- Error behavior is stable under equivalent input and profile.
- `query`/`trace` remain navigation tools; proof and review workflows live in `impacted`, `why`, and `doctor`.
