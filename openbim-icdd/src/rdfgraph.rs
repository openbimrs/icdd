//! A tiny subject-indexed view over an RDF/XML file, built on `oxrdfxml`. The
//! ICDD graphs are small and star-shaped (ContainerDescription → documents;
//! Link → elements), so we don't need a full triple store or SPARQL — just
//! "give me the objects of predicate P on subject S" and "which subjects have
//! rdf:type T". This module is the ONLY place `oxrdf`/`oxrdfxml` types are
//! touched; everything above it works on the neutral IR in `model.rs`.

use super::error::IcddError;
use super::vocab;
use oxrdf::{NamedOrBlankNode, Term};
use std::collections::BTreeMap;
use std::io::Read;

/// One object value on a subject: either a resource (IRI/blank-node id) or a
/// literal string. We keep the raw string form; datatype/language are dropped
/// because the ICDD vocabulary is plain string/dateTime/anyURI/boolean.
#[derive(Debug, Clone)]
pub enum Obj {
    /// A resource reference — the subject id it points at.
    Resource(String),
    /// A literal value (the lexical form).
    Literal(String),
}

impl Obj {
    /// The literal string, if this is a literal.
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            Obj::Literal(s) => Some(s),
            _ => None,
        }
    }
    /// The referenced subject id, if this is a resource.
    pub fn as_resource(&self) -> Option<&str> {
        match self {
            Obj::Resource(s) => Some(s),
            _ => None,
        }
    }
}

/// Subject id → (predicate IRI → list of objects). Subject/resource ids are the
/// full IRI for named nodes and `_:label` for blank nodes.
#[derive(Debug, Default)]
pub struct RdfGraph {
    /// Subject → (predicate, object) pairs.
    ///
    /// **`BTreeMap`, not `HashMap`, on purpose.** `subjects_of_type` iterates
    /// this map and returns the subjects in iteration order, which flows
    /// straight into `solibri icdd` output and any caller listing container
    /// documents. With a `HashMap` that order changed between runs of the same
    /// binary, so the same `.icdd` produced differently-ordered listings —
    /// the same class of reproducibility bug as the `.smc` writer's
    /// `typed_elems` map (PLAN T1).
    by_subject: BTreeMap<String, Vec<(String, Obj)>>,
}

impl RdfGraph {
    /// Parse an RDF/XML byte stream into the subject-indexed graph. Lenient:
    /// the whole file is consumed; a malformed triple aborts with an error
    /// (the ICDD conformance gate is elsewhere).
    pub fn parse<R: Read>(reader: R) -> Result<Self, IcddError> {
        let mut graph = RdfGraph::default();
        for (index, triple) in oxrdfxml::RdfXmlParser::new().for_reader(reader).enumerate() {
            if index == crate::rdf::MAX_RDF_TRIPLES {
                return Err(IcddError::NotConformant(format!(
                    "RDF graph exceeds the {}-triple limit",
                    crate::rdf::MAX_RDF_TRIPLES
                )));
            }
            let triple = triple.map_err(|error| IcddError::Rdf(error.to_string()))?;
            let subject = match triple.subject {
                NamedOrBlankNode::NamedNode(n) => n.into_string(),
                NamedOrBlankNode::BlankNode(b) => format!("_:{}", b.as_str()),
            };
            let predicate = triple.predicate.into_string();
            let object = match triple.object {
                Term::NamedNode(node) => Obj::Resource(node.into_string()),
                Term::BlankNode(node) => Obj::Resource(format!("_:{}", node.as_str())),
                Term::Literal(literal) => Obj::Literal(literal.value().to_string()),
                // rdf-12 quoted triples: not used by ICDD.
                #[allow(unreachable_patterns)]
                _ => continue,
            };
            graph
                .by_subject
                .entry(subject)
                .or_default()
                .push((predicate, object));
        }
        Ok(graph)
    }

    /// All subjects with an exact `rdf:type` IRI in `namespace`.
    pub fn subjects_of_type_ns(&self, namespace: &str, type_local: &str) -> Vec<&str> {
        let expected = format!("{namespace}{type_local}");
        self.by_subject
            .iter()
            .filter_map(|(subject, pairs)| {
                pairs
                    .iter()
                    .any(|(predicate, object)| {
                        predicate == vocab::RDF_TYPE
                            && matches!(object, Obj::Resource(iri) if iri == &expected)
                    })
                    .then_some(subject.as_str())
            })
            .collect()
    }

    /// True if `subject` has the exact namespaced RDF type.
    pub fn has_type_ns(&self, subject: &str, namespace: &str, type_local: &str) -> bool {
        let expected = format!("{namespace}{type_local}");
        self.objects(subject, vocab::RDF_TYPE)
            .iter()
            .any(|object| matches!(object, Obj::Resource(iri) if iri == &expected))
    }

    /// The objects of `predicate_iri` on `subject` (empty if none).
    pub fn objects(&self, subject: &str, predicate_iri: &str) -> Vec<&Obj> {
        self.by_subject
            .get(subject)
            .map(|po| {
                po.iter()
                    .filter(|(p, _)| p == predicate_iri)
                    .map(|(_, o)| o)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Objects for an exact ontology predicate.
    pub fn objects_ns(&self, subject: &str, namespace: &str, pred_local: &str) -> Vec<&Obj> {
        self.objects(subject, &format!("{namespace}{pred_local}"))
    }

    /// First literal for an exact ontology predicate.
    pub fn literal_ns(&self, subject: &str, namespace: &str, pred_local: &str) -> Option<String> {
        self.objects_ns(subject, namespace, pred_local)
            .into_iter()
            .find_map(|object| object.as_literal().map(str::to_string))
    }

    /// First resource for an exact ontology predicate.
    pub fn resource_ns(&self, subject: &str, namespace: &str, pred_local: &str) -> Option<String> {
        self.objects_ns(subject, namespace, pred_local)
            .into_iter()
            .find_map(|object| object.as_resource().map(str::to_string))
    }
}
