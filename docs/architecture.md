# Architecture

## Repository role

`openbimrs/icdd` is the single implementation repository for ICDD.
`openbimrs/openbim` pins it at `packages/icdd`; Solibri-rs exposes compatibility
adapters; Poing consumes those adapters. Neither consumer owns a second archive
or RDF implementation.

The child repository remains independently buildable and publishable.

## Package identity

```text
icdd  -- exact-version dependency -->  openbim-icdd
(alias; no types)                      (all ICDD behavior)
```

Cargo has no publisher-side package alias. The short package therefore contains
only:

```rust
pub use openbim_icdd::*;
```

Every public type originates in `openbim-icdd`. The alias dependency uses an
exact `=` requirement so canonical and alias releases cannot drift.

## Domain and implementation boundaries

```text
core  <-  IFC
                 +--> zip 8.x
openbim-icdd ----+--> oxrdfxml 0.2.x --> quick-xml (transitive)
                 +--> oxrdf 0.3.x

openbim facade ------> openbim-icdd
Solibri codec adapter -> openbim-icdd
Poing runtime --------> Solibri adapter -> openbim-icdd
```

- ICDD owns ISO 21597 archive paths, typed index/linkset views, structural
  conformance reporting, deterministic construction, and extension handling.
- `zip`, XML, and RDF syntax are implementation technologies supplied by
  maintained ecosystem crates.
- There is no dependency on `openbim-codec-xml` or `openbim-codec-zip`.
- Payload IFC, PDF, spreadsheet, drawing, and image bytes remain opaque.
- IFC and core crates must never depend on ICDD; other standards must not depend
  on ICDD merely because ICDD can carry their files.
- RDF remains inside ICDD until another real consumer justifies extraction.
- The `openbim` facade may optionally re-export ICDD.

The raw RDF API intentionally exposes Oxigraph triple types. That avoids a
second home-grown RDF model and lets extension graphs preserve unknown triples
semantically through parse/serialize cycles. This is a semantic guarantee, not
lexical byte identity: RDF/XML namespace prefixes, whitespace, and triple order
may change, and deterministic archive construction does not reproduce the input
ZIP envelope. Opaque payload bytes remain exact.

## Reader policy

The reader decodes `Index.rdf` and referenced linksets eagerly while retaining
lazy access to payload bytes in the ZIP archive. It applies explicit uncompressed
size limits to RDF metadata to stop compressed metadata bombs. Payloads may be
large by design and are read only on request.

Parsing is lenient enough to inspect imperfect containers. Structural issues are
reported by `IcddContainer::conformance_issues`; successful parsing is not a
claim of complete normative ISO validation.

Filesystem extraction rejects absolute, parent-directory, and platform-prefix
paths before joining an archive name to an output directory.

## Writer policy

`IcddArchiveBuilder` accepts RDF/XML and opaque payloads, validates metadata
before writing, validates all archive-relative paths, rejects duplicate entries,
and emits deterministic entry ordering and timestamps. RDF/XML generation uses
`oxrdfxml`; no XML string concatenation is required by consumers.

The Poing federation extension is isolated from the ISO core behind explicitly
named `PoingFederation*` types and functions. Solibri converts its neutral model
to these transport types rather than moving Solibri model ownership into ICDD.

## Standards artifacts

No ISO, DIN, CEN, or other restricted standards material is tracked. Local
references live under ignored `references/`; only fixtures with known compatible
redistribution rights may enter version control.

## Cross-repository delivery

Changes spanning repositories follow dependency order:

1. verify, review, land, and publish `openbim-icdd`;
2. publish the exact-version `icdd` alias;
3. replace consumer implementations with the released canonical dependency;
4. update the `openbim` submodule pin;
5. verify consumers from clean trees and fresh registry resolution.

Each submodule/dependency pin is a compatibility declaration and rollback point.
