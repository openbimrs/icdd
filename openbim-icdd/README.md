# openbim-icdd

Canonical Rust package for ISO 21597 Information Container for linked Document
Delivery (ICDD).

ICDD is an open federation container: a ZIP carries payload documents unchanged
while RDF describes the container and links elements across those documents.
The crate is deliberately payload-model-agnostic.

## Status

**Reserved scaffold.** Version `0.1.0` establishes package ownership and the
conventional archive path constants. It does not parse, validate, or write ICDD.
See the [repository status table](https://github.com/openbimrs/icdd#status) for
precise capability claims.

## Package names

This is the canonical implementation and owns every type. The sibling
[`icdd`](https://crates.io/crates/icdd) package is an exact-version pure
re-export so users can choose either crates.io name without creating duplicate
implementations.

## Standards material

No ISO, DIN, CEN, or other restricted artifact is packaged. Local material
belongs under the repository's ignored `references/` directory.

## License

MIT
