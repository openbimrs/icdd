# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Typed RDF/XML writing: `ContainerIndex::to_rdf_xml`, `LinkSet::to_rdf_xml`,
  `IcddArchiveBuilder::from_index` and `IcddArchiveBuilder::add_typed_linkset`
  encode documents (Internal, External, Folder, with the Secured and Encrypted
  mix-ins and `ct:requested`), links (`ls:Link`, `ls:DirectedLink`,
  `ls:Directed1toNLink`) and String, URI and Query identifiers, so consumers no
  longer assemble `ct:`/`ls:` triples. Output goes through `serialize_rdf_xml`
  and is deterministic. Invalid input returns `IcddError::NotConformant`:
  duplicate ids, duplicate from/to ids or link elements, link elements whose
  document is not in the index, links with fewer than two elements, directed
  links without a from or a to element, query identifiers missing their
  language or expression, blank-node labels that are not XML NCNames, a
  `ct:creationDate` that is not an `xsd:dateTime`, and control characters XML
  1.0 cannot carry.
- `add_typed_linkset` takes only the linkset, not the issue's
  `(linkset, index, base_iri)`: `from_index` stores the typed index and base in
  the builder and the linkset is validated against them. On a builder made with
  `new` it returns `NotConformant`.
- `PartialEq` and `Eq` for the schema types.

### Changed

- `conformance_issues` is stricter: it also reports a linkset element whose
  `ls:hasDocument` is not a document of the index, including for linksets
  added with `add_linkset`.
- The Poing federation writer builds its `Index.rdf` through the typed encoder.
  Its `Index.rdf` bytes change: `ct:url` is now typed `xsd:anyURI` and the
  prefix declarations differ (`ct` only; `ls` is declared only in linkset
  files). The RDF triples are otherwise equivalent.

## [0.2.1] - 2026-10-28

### Changed

- **Licence:** `AGPL-3.0-or-later OR LicenseRef-Commercial`; the last
  published release (0.2.0) was `MIT`. A commercial licence is available from
  the copyright holder named in `NOTICE`. See `LICENSING.md`. The crates ship
  `LICENSE`, `NOTICE` and `LICENSES/LicenseRef-Commercial.txt`.
- Raise MSRV to Rust 1.99.
- Relicensed repository-authored work from MIT to `AGPL-3.0-or-later`; historical releases remain under their published MIT terms, and third-party material retains its own terms.
- Clarified that structural checks are not complete normative ISO certification
  and documented the exact payload/RDF semantic preservation boundary without
  implying byte-identical RDF/XML or ZIP-envelope reconstruction.
- Removed application-specific consumer state from the public capability table.

## [0.2.0] - 2026-08-25

### Added

- Lazy ICDD ZIP reading with typed container-index, document, and linkset views.
- Payload access and traversal-safe filesystem extraction.
- Explicit uncompressed size limits for RDF metadata.
- Deterministic generic archive construction for payloads, linksets, and
  ontology resources.
- Raw RDF/XML parsing and serialization through `oxrdfxml` and `oxrdf`, including
  semantic preservation tests for unknown triples.
- Deterministic Poing federation-extension read/write support.
- Synthetic reader, writer, security, RDF round-trip, and federation tests.
- Compatibility verification against Solibri's three existing ICDD oracle files.

### Changed

- Replaced the retired shared archive-wrapper direction with direct maintained
  `zip`, `oxrdfxml`, and `oxrdf` dependencies plus ICDD-owned archive safety and
  selection policy; no first-party XML or ZIP codec crate is used.
- Extracted the ICDD family into its canonical standalone repository while
  preserving its OpenBIM.rs history.
- Made this repository the sole ICDD implementation boundary; Solibri and Poing
  consumer migrations are maintained as separate downstream changes.
- Made package and dependency metadata independent of the integration workspace.
- Added standalone documentation, CI, package verification, and an executable
  semantic purity gate for the `icdd` alias package. The gate resolves Cargo's
  active target and dependency metadata, enforces lockstep package versions,
  and rejects alternate implementation targets or files, target-gated
  dependencies, dependency-level feature overrides, or textual manifest decoys.
- Bumped both package names in lockstep to `0.2.0`.

## [0.1.0] - 2026-08-24

### Added

- Reserved the `openbim-icdd` and `icdd` package names.
- Added conventional ICDD archive path constants.
- Established `icdd` as a pure re-export of the canonical package.

[Unreleased]: https://github.com/openbimrs/icdd/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/icdd/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/openbimrs/icdd/compare/v0.1.0...v0.2.0
[0.1.0]: https://crates.io/crates/openbim-icdd/0.1.0
