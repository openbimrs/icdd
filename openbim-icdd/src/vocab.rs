//! IRI constants for the two ISO 21597-1 ontologies plus the annotation
//! vocabularies (Dublin Core, FOAF) that conformant containers use for
//! provenance. Namespaces verified against the authoritative ontologies at
//! <https://standards.iso.org/iso/21597/-1/ed-1/en/Container.rdf> and
//! `.../Linkset.rdf`, and against the official AnnexA reference containers.

/// Container ontology namespace (`ct:`).
pub const CT: &str = "https://standards.iso.org/iso/21597/-1/ed-1/en/Container#";
/// Linkset ontology namespace (`ls:`).
pub const LS: &str = "https://standards.iso.org/iso/21597/-1/ed-1/en/Linkset#";
/// RDF syntax namespace.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// `rdf:type`.
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// The mandatory value of `ct:conformanceIndicator` for a Part-1 container.
pub const CONFORMANCE_INDICATOR: &str = "ICDD-Part1-Container";

/// Build a `ct:` term IRI.
#[inline]
pub fn ct(local: &str) -> String {
    format!("{CT}{local}")
}
/// Build an `ls:` term IRI.
#[inline]
pub fn ls(local: &str) -> String {
    format!("{LS}{local}")
}

/// The local name of a `ct:`/`ls:`/other IRI (segment after the last `#` or `/`).
/// Used to dispatch `rdf:type` and predicates independently of the (mixed-case,
/// per-container) base namespace.
#[inline]
pub fn local_name(iri: &str) -> &str {
    match iri.rsplit_once('#') {
        Some((_, l)) => l,
        None => iri.rsplit_once('/').map(|(_, l)| l).unwrap_or(iri),
    }
}

/// True if `iri` is a term in the Container ontology namespace.
#[inline]
pub fn is_ct(iri: &str) -> bool {
    iri.starts_with(CT)
}
/// True if `iri` is a term in the Linkset ontology namespace.
#[inline]
pub fn is_ls(iri: &str) -> bool {
    iri.starts_with(LS)
}
