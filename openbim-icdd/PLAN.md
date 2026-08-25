# openbim-icdd implementation plan

Status: canonical implementation in progress.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md`; record blockers and
evidence here, and check work off only with executable proof.

## Goal

Make this repository the sole generic ICDD implementation. Solibri and Poing
must consume this crate rather than maintaining their own archive/RDF parser.

## Established boundary

- Depend directly on maintained `zip`, `oxrdfxml`, and `oxrdf`; do not create or
  consume hand-written XML/ZIP codec crates.
- RDF remains an ICDD concern. Payload bytes remain model-agnostic.
- Solibri-specific IFC-to-SMC conversion remains in Solibri after canonical ICDD
  extracts payloads.
- Solibri/Poing federation extensions may build on a generic canonical writer;
  they must not duplicate ZIP, Index.rdf, linkset, or RDF/XML machinery.
- Preserve the existing public reader contract where sound, but fix unsafe path
  extraction, duplicate/archive ambiguity, and unbounded metadata reads.

## Workstreams

- [x] `ICD-OPEN` — ZIP reader, lazy payload access, neutral Index.rdf IR
- [x] `ICD-LINKSET` — decode linkset RDF graphs through `oxrdfxml`
- [x] `ICD-WRITE` — deterministic generic ICDD writer using `zip` and
  `oxrdfxml`, including extension RDF payloads
- [x] `ICD-SECURITY` — canonical ZIP names, duplicate/case-fold rejection,
  exact ontology membership, traversal/symlink-safe extraction, bounded archive,
  metadata, RDF-triple, payload, linkset, and extraction resources
- [x] `ICD-ROUNDTRIP` — preserve unknown RDF semantics and untouched payload bytes
- [ ] `ICD-MIGRATE` — make Solibri depend on this crate and remove its generic
  ICDD module; make Poing use that canonical implementation path
- [x] `ICD-CONFORMANCE` — synthetic redistributable fixtures plus optional local
  ISO corpus checks where redistribution is not established

## Validation

1. Write consumer-compatible tests before implementation and observe failure.
2. Run standalone format/build/test/Clippy/rustdoc/package gates on Rust 1.88.
3. Mutation-probe alias purity and canonical-dependency enforcement.
4. Run Solibri ICDD, CLI, and Python tests against the canonical crate.
5. Run Poing importer tests against the migrated binding/fallback path.
6. Independently review exact clean commits before child-first landing.

## Risks and rollback

- Solibri and Poing have concurrent dirty work. Do not edit or reset those shared
  trees. Consumer migration must be prepared in isolated snapshots/worktrees and
  landed only without overwriting current work.
- Do not copy restricted `references/` material into this public repository.
- Publish canonical `openbim-icdd` before any exact registry dependency and then
  publish the `icdd` alias in lockstep.

## Evidence

- Standalone reader/writer/security/RDF/federation tests: 30 passed, plus one
  compile-tested doctest.
- Existing Solibri ICDD oracle and reproducibility suites against the hardened
  canonical crate: 6 passed both in the isolated harness and clean consumer.
- Alias purity: baseline accepted and all mutations rejected.
- Full locked constituent gate: format, build, tests, Clippy, rustdoc, alias
  mutations, and canonical package verification pass. Alias packaging follows
  canonical `openbim-icdd = 0.2.0` publication by Cargo's required order.
- Follow-up security regressions cover namespace spoofing, uncontained members,
  subtype-only links, undeclared/missing linksets, case-folded ZIP names,
  noncanonical paths, unavailable/ambiguous federation sources, undeclared
  writer inputs, source-binding mismatches, UUIDs, and robust affine/invertible
  transforms.
- Restricted ISO material was not copied into the repository.
