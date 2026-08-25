//! RDF/XML parsing and serialization backed by Oxigraph's maintained crates.
//!
//! ICDD-specific typed views are layered above this module. These functions are
//! public so extension graphs can remain in their owning domain while all RDF/XML
//! bytes are still handled by one proven implementation.

use std::io::Read;

pub use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};

use crate::IcddError;

/// Deterministic RDF/XML serialization settings.
#[derive(Debug, Clone, Default)]
pub struct RdfXmlOptions {
    base_iri: Option<String>,
    prefixes: Vec<(String, String)>,
}

impl RdfXmlOptions {
    /// Empty options with no base IRI or custom prefixes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the document base IRI.
    #[must_use]
    pub fn with_base_iri(mut self, iri: impl Into<String>) -> Self {
        self.base_iri = Some(iri.into());
        self
    }

    /// Register a readable namespace prefix.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>, iri: impl Into<String>) -> Self {
        self.prefixes.push((prefix.into(), iri.into()));
        self
    }
}

/// Parse a complete RDF/XML document without discarding unknown triples,
/// literal datatypes, language tags, or blank nodes.
pub fn parse_rdf_xml(reader: impl Read) -> Result<Vec<Triple>, IcddError> {
    oxrdfxml::RdfXmlParser::new()
        .for_reader(reader)
        .map(|triple| triple.map_err(|error| IcddError::Rdf(error.to_string())))
        .collect()
}

/// Serialize an RDF graph with Oxigraph's RDF/XML serializer.
///
/// Input triples are sorted first, making output independent of caller insertion
/// order. RDF semantics are preserved; lexical XML layout is intentionally not a
/// round-trip contract.
pub fn serialize_rdf_xml(triples: &[Triple], options: RdfXmlOptions) -> Result<Vec<u8>, IcddError> {
    let mut config = oxrdfxml::RdfXmlSerializer::new();
    if let Some(base_iri) = options.base_iri {
        config = config
            .with_base_iri(base_iri)
            .map_err(|error| IcddError::Rdf(error.to_string()))?;
    }
    for (prefix, iri) in options.prefixes {
        config = config
            .with_prefix(prefix, iri)
            .map_err(|error| IcddError::Rdf(error.to_string()))?;
    }

    let mut ordered = triples.iter().collect::<Vec<_>>();
    ordered.sort_by_cached_key(ToString::to_string);

    let mut serializer = config.for_writer(Vec::new());
    for triple in ordered {
        serializer
            .serialize_triple(triple.as_ref())
            .map_err(|error| IcddError::Rdf(error.to_string()))?;
    }
    serializer
        .finish()
        .map_err(|error| IcddError::Rdf(error.to_string()))
}
