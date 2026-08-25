# ICDD repository instructions

This repository owns the OpenBIM.rs implementation of ISO 21597 Information
Container for linked Document Delivery and its short-name package alias. The
published crates are reserved scaffolds; do not describe archive parsing, RDF
handling, validation, or writing as implemented without executable evidence.

## Map

- `openbim-icdd/` — canonical implementation and all public type definitions
- `icdd/` — pure re-export alias; no implementation or independent types
- `docs/` — repository architecture and maintained documentation
- `scripts/gate.sh` — complete local/CI verification gate
- `CHANGELOG.md` — user-visible changes using Keep a Changelog
- `references/` — ignored local standards corpus; never publish implicitly

## Commands

```bash
./scripts/gate.sh
cargo test --workspace
cargo package -p openbim-icdd
cargo package -p icdd
```

Trust command exit codes. Never summarize a Cargo pipeline in a way that hides
the Cargo process status.

## Boundaries

- `openbim-icdd` may depend on released `openbim-core`, ZIP framing, and public
  IFC contracts where the standard requires them.
- IFC, core, and codec crates must never depend on ICDD.
- RDF remains an ICDD concern until another real consumer justifies extraction.
- `icdd/src/lib.rs` must contain only `pub use openbim_icdd::*;`.
- Cross-repository dependency versions are explicit in crate manifests; do not
  replace them with parent-workspace inheritance.
- Do not vendor ISO, DIN, CEN, or other restricted standards material without
  verified redistribution rights.

## Documentation discipline

Keep capability tables honest: distinguish reserved API, implemented algorithm,
and conformance-tested behavior. Update README, rustdoc, and `CHANGELOG.md`
together for user-visible changes.
