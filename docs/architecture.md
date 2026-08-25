# Architecture

## Repository role

`openbimrs/icdd` is the canonical source repository for the ICDD family.
`openbimrs/openbim` pins a verified commit at `packages/icdd` and provides
ecosystem-level integration tests and the feature-gated `openbim` facade.

The child repository must remain buildable without the integration workspace.
Published crates therefore use explicit package metadata and versioned registry
dependencies, not inheritance from a parent workspace.

## Package identity

```text
icdd  -- exact-version dependency -->  openbim-icdd  -->  openbim-core
(alias; no types)                      (all behavior)
```

Cargo permits consumers to rename a dependency locally, but crates.io has no
publisher-side alias facility. Reserving both `openbim-icdd` and `icdd` requires
two package records. The short package contains only:

```rust
pub use openbim_icdd::*;
```

This is not duplicated implementation. Every public type originates in
`openbim-icdd`, so dependency graphs that encounter both names still have one
type identity. The alias dependency uses an exact `=` version requirement to
prevent canonical and alias releases from drifting.

## Dependency direction

```text
core  <-  IFC  <-  ICDD  ->  zip / RDF mechanics

openbim facade  ->  ICDD
```

- ICDD may consume shared vocabulary, ZIP framing, and public IFC contracts.
- IFC and core crates must never depend on ICDD.
- RDF remains inside ICDD until another real consumer justifies extraction.
- The `openbim` facade may optionally re-export ICDD.

ICDD remains payload-model-agnostic: carrying an IFC or PDF does not require the
container crate to parse it.

## Workspace independence

Package version, edition, MSRV, license, authors, repository, and
cross-repository dependency versions are explicit in each published manifest.
The parent integration workspace substitutes its local `openbim-core` through a
`[patch.crates-io]` entry, guaranteeing one package identity while exercising
the exact pinned child commit.

## Standards artifacts

This repository does not track ISO, DIN, CEN, or other restricted standards
material. Local references live under ignored `references/`. Conformance
fixtures require known redistribution rights before admission.

## Cross-repository delivery

Changes spanning repositories follow dependency order:

1. land and publish lower-level contract changes;
2. update and verify `openbim-icdd` standalone;
3. publish `openbim-icdd`;
4. publish the exact-version `icdd` alias;
5. update and verify the `openbim` submodule pin;
6. publish the integration commit.

The superproject pin is the compatibility declaration and rollback point.
