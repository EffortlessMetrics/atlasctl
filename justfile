default:
    cargo run -p xtask -- ci-fast

ci-fast:
    cargo run -p xtask -- ci-fast

ci-full:
    cargo run -p xtask -- ci-full

smoke:
    cargo run -p xtask -- smoke

docs-check:
    cargo run -p xtask -- docs-check
