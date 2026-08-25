#!/usr/bin/env bash
# Alias package must be a pure re-export of the exact canonical version.
set -euo pipefail

cd "$(dirname "$0")/.."

lib="icdd/src/lib.rs"
manifest="icdd/Cargo.toml"

non_comments="$(grep -vE '^\s*(//|$)' "$lib")"
if [ "$non_comments" != "pub use openbim_icdd::*;" ]; then
    printf '%s\n' 'icdd/src/lib.rs must contain only: pub use openbim_icdd::*;' >&2
    exit 1
fi

if ! grep -qE 'openbim-icdd = \{ path = "\.\./openbim-icdd", version = "=[0-9]' "$manifest"; then
    printf '%s\n' 'icdd must path-depend on an exact (=) openbim-icdd version' >&2
    exit 1
fi
