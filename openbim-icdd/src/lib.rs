//! `openbim-icdd` — ISO 21597 Information Container for linked Document Delivery.
//!
//! # What this is
//!
//! The open ISO federation container: a ZIP holding payload documents
//! untouched (IFC, PDF, XLSX, DWG, images) plus RDF describing which documents
//! are inside (`Index.rdf`) and how elements across them link
//! (`Payload triples/*.rdf`).
//!
//! # It is a front-end, not a model
//!
//! An ICDD's geometry lives inside its payload IFC files. This crate opens the
//! container, decodes the RDF into a neutral form and yields payload bytes —
//! it must **not** depend on `ifc-model`. Keeping it model-agnostic is what
//! lets an ICDD carry documents this workspace cannot parse at all.
//!
//! # Two layers, one format
//!
//! ZIP framing comes from `openbim-codec-zip`. The RDF layer is deliberately **not** in
//! `wire-*`: ICDD is this workspace's only RDF consumer, and putting an RDF
//! stack down there would make every `openbim-ids` user compile it.
//!
//! # Status
//!
//! **Reserved — no implementation.** Published to establish the name.

#![forbid(unsafe_code)]

/// Conventional path of the container index inside an ICDD archive.
///
/// Fixed by the standard; a ZIP without it is not an ICDD, which is the
/// cheapest available discriminator against a plain archive.
pub const INDEX_PATH: &str = "Index.rdf";

/// Conventional directory holding payload documents.
pub const PAYLOAD_DOCUMENTS_DIR: &str = "Payload documents";

/// Conventional directory holding linkset RDF graphs.
pub const PAYLOAD_TRIPLES_DIR: &str = "Payload triples";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_paths_are_the_standard_ones() {
        assert_eq!(INDEX_PATH, "Index.rdf");
        // Both payload directories contain a space; quoting them in shell or
        // ZIP tooling is a recurring source of mistakes.
        assert!(PAYLOAD_DOCUMENTS_DIR.contains(' '));
        assert!(PAYLOAD_TRIPLES_DIR.contains(' '));
    }
}
