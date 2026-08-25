#!/usr/bin/env bash
# Semantic alias-package boundary check; Cargo metadata is the source of truth.
set -euo pipefail

cd "$(dirname "$0")/.."
exec python3 scripts/check_alias_purity.py
