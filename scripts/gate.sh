#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/icdd.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo build --locked --workspace --all-targets
cargo test --locked --workspace --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
scripts/check-alias-purity.sh
python3 scripts/test_alias_purity.py
cargo package --locked -p openbim-icdd
cargo package --locked -p icdd
