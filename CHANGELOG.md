# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Extracted the ICDD family into its canonical standalone repository while
  preserving its OpenBIM.rs history.
- Made package and dependency metadata independent of the integration workspace.
- Added standalone documentation, CI, package verification, and an executable
  semantic purity gate for the `icdd` alias package. The gate resolves Cargo's
  active target and dependency metadata, enforces lockstep package versions,
  and rejects alternate implementation targets, target-gated dependencies,
  dependency-level feature overrides, or textual manifest decoys.

## [0.1.0] - 2026-08-24

### Added

- Reserved the `openbim-icdd` and `icdd` package names.
- Added conventional ICDD archive path constants.
- Established `icdd` as a pure re-export of the canonical package.

[Unreleased]: https://github.com/openbimrs/icdd/commits/main
[0.1.0]: https://crates.io/crates/openbim-icdd/0.1.0
