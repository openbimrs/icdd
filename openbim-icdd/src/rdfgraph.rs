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
        let mut g = RdfGraph::default();
        for t in oxrdfxml::RdfXmlParser::new().for_reader(reader) {
            let t = t.map_err(|e| IcddError::Rdf(e.to_string()))?;
            let subj = match t.subject {
                NamedOrBlankNode::NamedNode(n) => n.into_string(),
                NamedOrBlankNode::BlankNode(b) => format!("_:{}", b.as_str()),
            };
            let pred = t.predicate.into_string();
            let obj = match t.object {
                Term::NamedNode(n) => Obj::Resource(n.into_string()),
                Term::BlankNode(b) => Obj::Resource(format!("_:{}", b.as_str())),
                Term::Literal(l) => Obj::Literal(l.value().to_string()),
                // rdf-12 quoted triples: not used by ICDD.
                #[allow(unreachable_patterns)]
                _ => continue,
            };
            g.by_subject.entry(subj).or_default().push((pred, obj));
        }
        Ok(g)
    }

    /// All subject ids whose `rdf:type` local name equals `type_local`
    /// (e.g. `"InternalDocument"`, `"Link"`). Matched by local name so the
    /// per-container base namespace doesn't matter.
    pub fn subjects_of_type(&self, type_local: &str) -> Vec<&str> {
        let mut out = Vec::new();
        for (subj, po) in &self.by_subject {
            for (p, o) in po {
                if p == vocab::RDF_TYPE {
                    if let Obj::Resource(t) = o {
                        if vocab::local_name(t) == type_local {
                            out.push(subj.as_str());
                            break;
                        }
                    }
                }
            }
        }
        out
    }

    /// True if `subject` has an `rdf:type` whose local name equals `type_local`.
    pub fn has_type(&self, subject: &str, type_local: &str) -> bool {
        self.objects(subject, vocab::RDF_TYPE)
            .iter()
            .any(|o| matches!(o, Obj::Resource(t) if vocab::local_name(t) == type_local))
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

    /// The objects of a predicate identified by its LOCAL name in the given
    /// namespace-agnostic sense — matches any predicate whose local name equals
    /// `pred_local`. Used for `ct:`/`ls:` predicates without hardcoding the
    /// (fixed) namespace, tolerant of the two ISO namespace spellings.
    pub fn objects_local(&self, subject: &str, pred_local: &str) -> Vec<&Obj> {
        self.by_subject
            .get(subject)
            .map(|po| {
                po.iter()
                    .filter(|(p, _)| vocab::local_name(p) == pred_local)
                    .map(|(_, o)| o)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// First literal value of a local-named predicate on `subject`.
    pub fn literal(&self, subject: &str, pred_local: &str) -> Option<String> {
        self.objects_local(subject, pred_local)
            .into_iter()
            .find_map(|o| o.as_literal().map(str::to_string))
    }

    /// First resource reference of a local-named predicate on `subject`.
    pub fn resource(&self, subject: &str, pred_local: &str) -> Option<String> {
        self.objects_local(subject, pred_local)
            .into_iter()
            .find_map(|o| o.as_resource().map(str::to_string))
    }

    /// True if the subject has a boolean literal `true` for `pred_local`.
    pub fn bool_true(&self, subject: &str, pred_local: &str) -> bool {
        self.literal(subject, pred_local)
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }
}
