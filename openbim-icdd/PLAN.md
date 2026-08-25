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
- [x] `ICD-SECURITY` — traversal-safe extraction and bounded RDF metadata reads
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

- Standalone reader/writer/security/RDF/federation tests: 14 passed.
- Existing Solibri ICDD oracle suite against the canonical crate: 6 passed.
- Alias purity: baseline accepted and all mutations rejected.
- Full `scripts/gate.sh`: code, Clippy, rustdoc, and mutation stages pass;
  package verification requires the candidate to be committed and is run after
  the frozen commit is created.
- Restricted ISO material was not copied into the repository.
