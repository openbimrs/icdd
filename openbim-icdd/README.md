# openbim-icdd

Canonical Rust implementation of ISO 21597 Information Container for linked
Document Delivery (ICDD).

The crate reads ZIP containers lazily, decodes typed `Index.rdf` and linkset
views, exposes raw RDF/XML parse/serialize APIs for extensions, writes
deterministic containers, streams large payloads, and extracts opaque payloads
with traversal, symlink, and expansion guards. It uses the maintained `zip`,
`oxrdfxml`, and `oxrdf` crates directly.

`0.2.0` is the first functional release. Complete normative ISO validation and
byte-identical arbitrary archive rewriting are not yet claimed. See the
[repository status table](https://github.com/openbimrs/icdd#status) for the
precise capability boundary.

This package owns every ICDD type and behavior. The sibling
[`icdd`](https://crates.io/crates/icdd) package is an exact-version pure
re-export so users can choose either name without duplicate implementations.

No ISO, DIN, CEN, or other restricted artifact is packaged. Local material
belongs under the repository's ignored `references/` directory.

## License

MIT
