# OpenBIM.rs ICDD

[![CI](https://github.com/openbimrs/icdd/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/icdd/actions/workflows/ci.yml)
[![openbim-icdd](https://img.shields.io/crates/v/openbim-icdd.svg)](https://crates.io/crates/openbim-icdd)
[![icdd](https://img.shields.io/crates/v/icdd.svg)](https://crates.io/crates/icdd)
[![docs.rs](https://docs.rs/openbim-icdd/badge.svg)](https://docs.rs/openbim-icdd)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://www.rust-lang.org)

Pure-Rust infrastructure for ISO 21597 Information Container for linked
Document Delivery (ICDD): ZIP containers that preserve payload documents while
RDF describes the container and links elements across documents.

This repository is the canonical home of the ICDD family in
[OpenBIM.rs](https://github.com/openbimrs/openbim). The integration repository
pins this repository under `packages/icdd`.

## Status

The published `0.1.0` releases are **reserved scaffolds**, not an ICDD reader,
writer, or validator.

| Capability | Status |
| --- | --- |
| Conventional ICDD archive paths | Implemented |
| Two synchronized crates.io names | Implemented and structurally gated |
| ZIP container reading/writing | Not implemented |
| Index RDF decoding/encoding | Not implemented |
| Linkset RDF decoding/encoding | Not implemented |
| ISO 21597 validation | Not implemented |
| Lossless unknown-data round-trip | Not implemented |

No parser, writer, or validation capability should be inferred from the crates
existing on crates.io.

## Crates

| Package | Purpose |
| --- | --- |
| [`openbim-icdd`](openbim-icdd/) | Canonical implementation; owns every type and behavior |
| [`icdd`](icdd/) | Pure re-export alias pinned to the exact canonical version |

Cargo has dependency renaming but no crates.io package aliases. Two package
records are therefore required to reserve both names. `icdd` defines nothing of
its own and re-exports `openbim-icdd`, so both names expose the same types rather
than compiling duplicate implementations.

## Install

Use either package name:

```bash
cargo add openbim-icdd
# or
cargo add icdd
```

```rust
use openbim_icdd::INDEX_PATH;
// The short package exposes the same item as `icdd::INDEX_PATH`.
assert_eq!(INDEX_PATH, "Index.rdf");
```

Do not depend on both names directly. The alias already brings in the canonical
package at an exact version.

## Architecture

- [`docs/architecture.md`](docs/architecture.md) — repository, dependency, and alias boundaries
- [`openbimrs/openbim`](https://github.com/openbimrs/openbim) — integrated workspace and facade
- [`openbim-core`](https://crates.io/crates/openbim-core) — shared openBIM vocabulary

ICDD is deliberately model-agnostic. Payload IFC, PDF, spreadsheet, drawing, or
image bytes remain payloads; the ICDD layer handles container and link metadata
without requiring every payload format to be understood.

## Standards material

No ISO, DIN, CEN, or other restricted standards artifact is tracked or packaged.
Locally available references belong under ignored `references/`. A fixture may
enter version control only when its redistribution terms are known and compatible
with this repository.

## Development

Requires Rust `1.88` or newer.

```bash
git clone https://github.com/openbimrs/icdd.git
cd icdd
./scripts/gate.sh
```

The gate checks formatting, build, tests, Clippy, rustdoc, alias purity, and
crates.io package verification using command exit codes.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Capability work must add executable
evidence and update the status table without overstating coverage.

## License

MIT — see [`LICENSE`](LICENSE).
