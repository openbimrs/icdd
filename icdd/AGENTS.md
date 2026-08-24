# icdd instructions

Purpose: Alias crate: pure re-export so the standard is reachable under its common name.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Exactly one line of code: pub use <canonical>::*. Defining a type here is a defect -- see scripts/check-alias-purity.sh.

## Status

Published to reserve the name.
