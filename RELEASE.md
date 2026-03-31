# RELEASE

## Release bar

Before tagging a release:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -p atlasctl-cli -- build`
- `cargo run -p atlasctl-cli -- check --profile ci`

## Versioning

- bump schema only when the canonical JSON contract changes materially
- document node kind, edge kind, or profile changes in ADRs
- keep output ordering stable

## Artifacts

The first public artifact is source plus tags. Prebuilt binaries can come later with `cargo-dist`.
