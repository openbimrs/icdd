# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/icdd/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/icdd/compare/v0.1.0...v0.2.0
[0.1.0]: https://crates.io/crates/openbim-icdd/0.1.0
