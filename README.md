# OpenBIM.rs ICDD

[![CI](https://github.com/openbimrs/icdd/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/icdd/actions/workflows/ci.yml)
[![openbim-icdd](https://img.shields.io/crates/v/openbim-icdd.svg)](https://crates.io/crates/openbim-icdd)
[![icdd](https://img.shields.io/crates/v/icdd.svg)](https://crates.io/crates/icdd)
[![docs.rs](https://docs.rs/openbim-icdd/badge.svg)](https://docs.rs/openbim-icdd)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://www.rust-lang.org)

Pure-Rust infrastructure for ISO 21597 Information Container for linked
Document Delivery (ICDD): ZIP containers that preserve payload documents while
RDF describes the container and links elements across documents.

This repository is the **only implementation home** of ICDD in OpenBIM.rs.
Solibri-rs and Poing migration is tracked as consumer work: their end state is a
thin adapter over this crate, without private ZIP, RDF/XML, or ICDD parsers.

## Status

`0.1.x` reserved the package names. `0.2.0` is the first functional release.

| Capability | Status |
| --- | --- |
| Conventional ICDD archive paths | Implemented |
| Lazy ZIP container reading, bounded payload reads, and streaming copies | Implemented |
| Safe payload extraction | Implemented; rejects traversal and symlink paths and bounds expansion |
| Deterministic ZIP construction | Implemented |
| Typed `Index.rdf` decoding | Implemented |
| Typed ISO 21597-1 linkset decoding | Implemented |
| Raw RDF/XML parsing and serialization | Implemented; unknown triples survive semantic round-trip |
| Generic payload, linkset, and ontology writing | Implemented |
| Poing federation extension read/write | Implemented and deterministic |
| Structural conformance reporting | Implemented |
| Complete normative ISO 21597 validation | Not yet implemented |
| Byte-identical rewrite of arbitrary unknown ZIP/XML data | Not yet implemented |

The reader accepts documented compatibility casing where lookup remains unique,
but fails closed on ambiguous ZIP names, unsafe paths, malformed RDF, spoofed
ontology namespaces, and missing declared linksets. `conformance_issues` reports
non-fatal layout and indicator findings. This is not a claim of complete ISO
certification.

## Crates

| Package | Purpose |
| --- | --- |
| [`openbim-icdd`](openbim-icdd/) | Canonical implementation; owns every ICDD type and behavior |
| [`icdd`](icdd/) | Pure re-export alias pinned to the exact canonical version |

Cargo has dependency renaming but no crates.io package aliases. `icdd` defines
nothing and re-exports `openbim-icdd`, so both names expose identical types.
Do not depend on both names directly.

## Install

```bash
cargo add openbim-icdd
# or: cargo add icdd
```

```rust,no_run
use openbim_icdd::IcddContainer;

let mut container = IcddContainer::open_path("delivery.icdd")?;
for document in container.ifc_documents_including_requested() {
    println!("{}", document.name.as_deref().unwrap_or(&document.id));
}
# Ok::<(), openbim_icdd::IcddError>(())
```

Constructors accept RDF/XML produced by the maintained Oxigraph stack and write
the archive through the maintained `zip` crate:

```rust,no_run
use openbim_icdd::IcddArchiveBuilder;

let bytes = IcddArchiveBuilder::new(include_bytes!("Index.rdf"))?
    .add_payload("model.ifc", std::fs::read("model.ifc")?)?
    .finish()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Dependency boundary

ICDD uses maintained ecosystem libraries **directly**:

- [`zip`](https://crates.io/crates/zip) for archive I/O and Deflate;
- [`oxrdfxml`](https://crates.io/crates/oxrdfxml) for RDF/XML parsing and serialization;
- [`oxrdf`](https://crates.io/crates/oxrdf) for RDF terms and triples.

There is deliberately no first-party XML or ZIP abstraction crate in this
dependency path. ICDD is the domain boundary; XML and ZIP are implementation
technologies. The retired `openbim-codec-xml` scaffold is not used.

## Architecture

- [`docs/architecture.md`](docs/architecture.md) — ownership, dependencies, and alias boundaries
- [`openbimrs/openbim`](https://github.com/openbimrs/openbim) — integrated workspace and facade

Payload IFC, PDF, spreadsheet, drawing, or image bytes remain opaque. ICDD owns
container and link metadata without importing every payload format.

## Standards material

No ISO, DIN, CEN, or other restricted standards artifact is tracked or packaged.
Locally available references belong under ignored `references/`. A fixture may
enter version control only when its redistribution terms are known and compatible.
Synthetic fixtures cover CI; local oracle tests may use restricted references
without publishing them.

## Development

Requires Rust `1.88` or newer and Python `3.10` or newer for the semantic alias
gate.

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
