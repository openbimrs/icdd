//! `openbim-icdd` — ISO 21597 Information Container for linked Document Delivery.
//!
//! ICDD is a ZIP container holding opaque payload documents plus RDF/XML that
//! describes those documents and the links between their elements. This crate
//! owns that container boundary; it deliberately does not parse IFC, PDF, or
//! any other payload format.
//!
//! ZIP and RDF/XML are handled directly by maintained upstream crates (`zip`,
//! `oxrdfxml`, and `oxrdf`). OpenBIM.rs does not wrap those general formats in
//! home-grown XML or ZIP codec packages.
//!
//! # Implemented
//!
//! - deterministic ZIP reading and writing;
//! - neutral `Index.rdf` and linkset views;
//! - lazy, bounded payload access, streaming copies, and safe extraction;
//! - raw RDF graph parse/serialize APIs that preserve unknown RDF semantics;
//! - conformance diagnostics for the mandatory Part 1 layout.

#![forbid(unsafe_code)]

mod container;
mod error;
mod federation;
mod index;
mod linkset;
mod rdfgraph;
mod writer;

pub mod rdf;
pub mod read;
pub mod schema;
pub mod vocab;

pub use error::IcddError;
pub use federation::{
    parse_poing_federation_icdd, write_poing_federation_icdd, FederationIcddPayload,
    PoingFederationManifest, PoingFederationMember,
};
pub use rdf::{parse_rdf_xml, serialize_rdf_xml, RdfXmlOptions};
pub use read::IcddContainer;
pub use schema::*;
pub use writer::IcddArchiveBuilder;

/// Conventional path of the container index inside an ICDD archive.
pub const INDEX_PATH: &str = "Index.rdf";
/// Conventional directory holding ontology resources.
pub const ONTOLOGY_RESOURCES_DIR: &str = container::ONTOLOGY_DIR;
/// Conventional directory holding payload documents.
pub const PAYLOAD_DOCUMENTS_DIR: &str = container::PAYLOAD_DOCS_DIR;
/// Conventional directory holding linkset RDF graphs.
pub const PAYLOAD_TRIPLES_DIR: &str = container::PAYLOAD_TRIPLES_DIR;
