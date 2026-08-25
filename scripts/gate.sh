#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/icdd.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo fmt --all -- --check
cargo build --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
scripts/check-alias-purity.sh
cargo package -p openbim-icdd
cargo package -p icdd
