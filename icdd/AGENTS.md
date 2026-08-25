# icdd instructions

Purpose: short-name crates.io package alias for `openbim-icdd`.

Follow `../AGENTS.md`. Read `PLAN.md` for release coordination.

## Boundary

Exactly one code line is allowed:

```rust
pub use openbim_icdd::*;
```

Defining a type or behavior here is a defect. Keep the dependency pinned to the
exact canonical version; `scripts/check-alias-purity.sh` enforces both rules.

## Status

Published name reservation and pure re-export.
