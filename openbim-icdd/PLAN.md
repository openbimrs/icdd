# openbim-icdd implementation plan

Status: names reserved; implementation not started.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers and evidence under it, and check it off only with executable
proof.

## Established boundary

`openbim-core` plus ZIP framing when implemented. RDF remains inside this crate.
Payload bytes remain model-agnostic.

## Open work

- [ ] `ICD-OPEN` — open an ICDD container and decode `Index.rdf` into a neutral IR
- [ ] `ICD-LOSSLESS` — preserve unknown RDF and untouched payload bytes on round-trip
- [ ] `ICD-LINKSET` — decode and encode linkset graphs without collapsing unknown data
- [ ] `ICD-CONFORMANCE` — add redistributable conformance fixtures and validation evidence

## Completion log

Nothing completed yet. Record proof commands and results here when work is
checked off.
