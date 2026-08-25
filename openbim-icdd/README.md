# openbim-icdd

ISO 21597 ICDD (Information Container for linked Document Delivery) for Rust.

The open ISO federation container: a ZIP holding payload documents untouched (IFC, PDF, XLSX, DWG, images) plus RDF describing which documents are inside and how elements across them link.

Deliberately model-agnostic: it opens the container and yields payload bytes, but never builds an IFC model. An ICDD can carry documents this toolchain cannot parse at all.

## Status

**Reserved.** This release establishes the crate name and its place in the
layering. It does not parse files yet — see the crate documentation for what is
implemented versus reserved.

No ISO/CEN schema is vendored in this crate. Types are written *from* the
schemas; the schema files themselves are referenced out of tree, because
possessing a copy of a standard does not establish the right to redistribute it.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

Design rationale for the crate layout: `docs/adr/0015`.

## License

MIT
