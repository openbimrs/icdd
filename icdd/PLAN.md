# icdd alias plan

Status: published pure re-export; no implementation belongs here.
Last updated: 2026-08-25

## Invariant

`src/lib.rs` contains only:

```rust
pub use openbim_icdd::*;
```

The dependency stays pinned to the exact canonical version. The standalone and
integration gates enforce both conditions.

## Work queue

- [ ] `ALI-ICDD` — release only after the matching `openbim-icdd` version exists

There is no feature implementation queue for this package.
