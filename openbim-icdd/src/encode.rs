//! Typed encoders: [`ContainerIndex`] and [`LinkSet`] to RDF/XML.
//!
//! The inverse of `index::parse_index` and `linkset::parse_linkset`. Both
//! encoders build triples from the vocabulary in [`crate::vocab`] and hand them
//! to [`serialize_rdf_xml`](crate::serialize_rdf_xml), which sorts them, so the
//! bytes depend only on the values and never on insertion order. The rules for
//! callers are on [`ContainerIndex::to_rdf_xml`] and [`LinkSet::to_rdf_xml`].

use std::collections::BTreeSet;

use oxiri::Iri;
use oxrdf::vocab::xsd;

use crate::error::IcddError;
use crate::rdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, RdfXmlOptions, Term, Triple};
use crate::schema::{ContainerIndex, Document, DocumentKind, ElementIdentifier, Link, LinkSet};
use crate::vocab;

pub(crate) fn invalid(message: impl Into<String>) -> IcddError {
    IcddError::NotConformant(message.into())
}

pub(crate) fn named(iri: impl Into<String>) -> Result<NamedNode, IcddError> {
    NamedNode::new(iri.into()).map_err(|error| IcddError::Rdf(error.to_string()))
}

/// Serializer options for `Index.rdf`: the `ct` prefix only.
fn index_options(base_iri: &str) -> RdfXmlOptions {
    RdfXmlOptions::new()
        .with_base_iri(base_iri)
        .with_prefix("ct", vocab::CT)
}

/// Serializer options for a linkset file: the `ls` prefix only.
fn linkset_options(base_iri: &str) -> RdfXmlOptions {
    RdfXmlOptions::new()
        .with_base_iri(base_iri)
        .with_prefix("ls", vocab::LS)
}

/// XML 1.0 forbids C0 controls other than tab, LF and CR in character data.
fn check_xml_text(value: &str) -> Result<(), IcddError> {
    match value
        .chars()
        .find(|c| *c < ' ' && !matches!(c, '\t' | '\n' | '\r'))
    {
        Some(c) => Err(invalid(format!(
            "literal {value:?} contains the control character U+{:04X}, which XML 1.0 cannot carry",
            u32::from(c)
        ))),
        None => Ok(()),
    }
}

/// An XML NCName, as `rdf:nodeID` requires.
fn is_ncname(label: &str) -> bool {
    let mut chars = label.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// The lexical form of `xsd:dateTime`: `[-]YYYY-MM-DDThh:mm:ss[.s+][Z|(+|-)hh:mm]`.
fn is_xsd_date_time(value: &str) -> bool {
    fn digits(s: &str, n: usize) -> Option<u32> {
        (s.len() == n && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    }
    let value = value.strip_prefix('-').unwrap_or(value);
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    let mut date = date.rsplitn(3, '-');
    let (Some(day), Some(month), Some(year)) = (date.next(), date.next(), date.next()) else {
        return false;
    };
    if year.len() < 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let year_value: u64 = year.parse().unwrap_or(0);
    if year.len() > 4 && year.starts_with('0') || year_value == 0 {
        return false;
    }
    let (Some(month), Some(day)) = (digits(month, 2), digits(day, 2)) else {
        return false;
    };
    let leap = year_value.is_multiple_of(4)
        && (!year_value.is_multiple_of(100) || year_value.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > days {
        return false;
    }
    let (clock, zone) = match time.find(['Z', '+', '-']) {
        Some(at) => time.split_at(at),
        None => (time, ""),
    };
    let mut parts = clock.splitn(3, ':');
    let (Some(hour), Some(minute), Some(second)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let (whole, fraction) = match second.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (second, None),
    };
    if fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let (Some(hour), Some(minute), Some(whole)) =
        (digits(hour, 2), digits(minute, 2), digits(whole, 2))
    else {
        return false;
    };
    let end_of_day = hour == 24
        && minute == 0
        && whole == 0
        && fraction.is_none_or(|f| f.bytes().all(|b| b == b'0'));
    if !(end_of_day || (hour < 24 && minute < 60 && whole < 60)) {
        return false;
    }
    match zone {
        "" | "Z" => true,
        _ => {
            let Some((sign, offset)) = zone.split_at_checked(1) else {
                return false;
            };
            let Some((zh, zm)) = offset.split_once(':') else {
                return false;
            };
            matches!(sign, "+" | "-")
                && matches!((digits(zh, 2), digits(zm, 2)), (Some(h), Some(m)) if (h < 14 && m < 60) || (h == 14 && m == 0))
        }
    }
}

/// Resolves ids against the document base and rejects duplicates.
struct Ids<'a> {
    base: Iri<&'a str>,
    seen: BTreeSet<String>,
}

impl<'a> Ids<'a> {
    fn new(base_iri: &'a str) -> Result<Self, IcddError> {
        let base = Iri::parse(base_iri)
            .map_err(|error| invalid(format!("invalid base IRI {base_iri:?}: {error}")))?;
        Ok(Self {
            base,
            seen: BTreeSet::new(),
        })
    }

    /// The normalised form of `id` (absolute IRI, or `_:label`).
    fn resolve(&self, id: &str) -> Result<String, IcddError> {
        if id.is_empty() {
            return Err(invalid("empty node id"));
        }
        if let Some(label) = id.strip_prefix("_:") {
            if !is_ncname(label) {
                return Err(invalid(format!(
                    "blank node label {label:?} is not an XML NCName"
                )));
            }
            return Ok(id.to_string());
        }
        self.base
            .resolve(id)
            .map(Iri::into_inner)
            .map_err(|error| invalid(format!("invalid node id {id:?}: {error}")))
    }

    /// Resolve and claim `id`; a second claim is an error.
    fn claim(&mut self, id: &str, what: &str) -> Result<String, IcddError> {
        let resolved = self.resolve(id)?;
        if !self.seen.insert(resolved.clone()) {
            return Err(invalid(format!("duplicate node id {resolved} ({what})")));
        }
        Ok(resolved)
    }
}

fn subject(id: &str) -> Result<NamedOrBlankNode, IcddError> {
    match id.strip_prefix("_:") {
        Some(label) => BlankNode::new(label)
            .map(Into::into)
            .map_err(|error| invalid(format!("invalid blank node label {label:?}: {error}"))),
        None => Ok(named(id)?.into()),
    }
}

fn object(id: &str) -> Result<Term, IcddError> {
    Ok(match subject(id)? {
        NamedOrBlankNode::NamedNode(node) => node.into(),
        NamedOrBlankNode::BlankNode(node) => node.into(),
    })
}

struct Graph(Vec<Triple>);

impl Graph {
    fn add(
        &mut self,
        subject: &NamedOrBlankNode,
        predicate: String,
        object: impl Into<Term>,
    ) -> Result<(), IcddError> {
        self.0
            .push(Triple::new(subject.clone(), named(predicate)?, object));
        Ok(())
    }

    fn typed(&mut self, subject: &NamedOrBlankNode, class: String) -> Result<(), IcddError> {
        self.add(subject, vocab::RDF_TYPE.to_string(), named(class)?)
    }

    fn text(
        &mut self,
        subject: &NamedOrBlankNode,
        predicate: String,
        value: &str,
    ) -> Result<(), IcddError> {
        check_xml_text(value)?;
        self.add(subject, predicate, Literal::new_simple_literal(value))
    }

    fn typed_text(
        &mut self,
        subject: &NamedOrBlankNode,
        predicate: String,
        value: &str,
        datatype: oxrdf::NamedNodeRef<'_>,
    ) -> Result<(), IcddError> {
        check_xml_text(value)?;
        self.add(
            subject,
            predicate,
            Literal::new_typed_literal(value, datatype),
        )
    }

    fn optional_text(
        &mut self,
        subject: &NamedOrBlankNode,
        predicate: String,
        value: Option<&str>,
    ) -> Result<(), IcddError> {
        match value {
            Some(value) => self.text(subject, predicate, value),
            None => Ok(()),
        }
    }

    fn required_text(
        &mut self,
        subject: &NamedOrBlankNode,
        id: &str,
        local: &str,
        value: &str,
        ns: fn(&str) -> String,
    ) -> Result<(), IcddError> {
        if value.is_empty() {
            return Err(invalid(format!("{id} has an empty {local}")));
        }
        self.text(subject, ns(local), value)
    }
}

impl ContainerIndex {
    /// Encode this index as a deterministic `Index.rdf` RDF/XML document.
    ///
    /// Ids resolve against `base_iri`: a relative id such as `#doc1` becomes
    /// `{base_iri}#doc1`, the form the readers return. An id starting with `_:`
    /// is a blank node and its label must be an XML NCName.
    ///
    /// Reading the result back gives this index with its ids resolved and, as
    /// the reader's canonical order, its documents and linkset files sorted by
    /// id. Documents are typed Internal, External or Folder; a checksum adds
    /// `ct:SecuredDocument`, `encrypted` adds `ct:EncryptedDocument`, and
    /// `requested` writes `ct:requested`. The output is deterministic.
    ///
    /// # Errors
    ///
    /// [`IcddError::NotConformant`] for an invalid base IRI or id, a blank node
    /// label that is not an NCName, a duplicate id, a document with an empty
    /// `ct:filename`, `ct:url` or `ct:foldername`, a checksum with an empty
    /// field, a linkset file without a filename or with a duplicate filename, a
    /// `ct:creationDate` that is not an `xsd:dateTime`, or a literal containing
    /// a C0 control character other than tab, LF and CR (XML 1.0 cannot carry
    /// it).
    pub fn to_rdf_xml(&self, base_iri: &str) -> Result<Vec<u8>, IcddError> {
        let mut ids = Ids::new(base_iri)?;
        let mut graph = Graph(Vec::new());

        let root_id = ids.claim(&self.description.id, "container description")?;
        let root = subject(&root_id)?;
        graph.typed(&root, vocab::ct("ContainerDescription"))?;
        let description = &self.description;
        graph.optional_text(
            &root,
            vocab::ct("conformanceIndicator"),
            description.conformance_indicator.as_deref(),
        )?;
        graph.optional_text(
            &root,
            vocab::ct("description"),
            description.description.as_deref(),
        )?;
        if let Some(date) = description.creation_date.as_deref() {
            if !is_xsd_date_time(date) {
                return Err(invalid(format!(
                    "ct:creationDate {date:?} is not an xsd:dateTime"
                )));
            }
            graph.typed_text(&root, vocab::ct("creationDate"), date, xsd::DATE_TIME)?;
        }

        for document in &self.documents {
            let id = ids.claim(&document.id, "document")?;
            graph.add(&root, vocab::ct("containsDocument"), object(&id)?)?;
            encode_document(&mut graph, &id, document)?;
        }

        let mut filenames = BTreeSet::new();
        for file in &self.linkset_files {
            let id = ids.claim(&file.id, "linkset file")?;
            let filename = file
                .filename
                .as_deref()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| invalid(format!("linkset file {id} has no ct:filename")))?;
            if !filenames.insert(filename) {
                return Err(invalid(format!("duplicate linkset filename {filename}")));
            }
            let node = subject(&id)?;
            graph.add(&root, vocab::ct("containsLinkset"), object(&id)?)?;
            graph.typed(&node, vocab::ct("Linkset"))?;
            graph.text(&node, vocab::ct("filename"), filename)?;
            graph.optional_text(&node, vocab::ct("name"), file.name.as_deref())?;
        }

        crate::serialize_rdf_xml(&graph.0, index_options(base_iri))
    }
}

fn encode_document(graph: &mut Graph, id: &str, document: &Document) -> Result<(), IcddError> {
    let node = subject(id)?;
    match &document.kind {
        DocumentKind::Internal { filename } => {
            graph.typed(&node, vocab::ct("InternalDocument"))?;
            graph.required_text(&node, id, "filename", filename, vocab::ct)?;
        }
        DocumentKind::External { url } => {
            graph.typed(&node, vocab::ct("ExternalDocument"))?;
            if url.is_empty() {
                return Err(invalid(format!("{id} has an empty url")));
            }
            graph.typed_text(&node, vocab::ct("url"), url, xsd::ANY_URI)?;
        }
        DocumentKind::Folder { foldername } => {
            graph.typed(&node, vocab::ct("FolderDocument"))?;
            graph.required_text(&node, id, "foldername", foldername, vocab::ct)?;
        }
    }
    if let Some(checksum) = &document.checksum {
        if checksum.algorithm.is_empty() || checksum.value.is_empty() {
            return Err(invalid(format!("{id} has an empty checksum field")));
        }
        graph.typed(&node, vocab::ct("SecuredDocument"))?;
        graph.text(&node, vocab::ct("checksum"), &checksum.value)?;
        graph.text(&node, vocab::ct("checksumAlgorithm"), &checksum.algorithm)?;
    }
    if document.encrypted {
        graph.typed(&node, vocab::ct("EncryptedDocument"))?;
    }
    if document.requested {
        graph.typed_text(&node, vocab::ct("requested"), "true", xsd::BOOLEAN)?;
    }
    graph.optional_text(&node, vocab::ct("name"), document.name.as_deref())?;
    graph.optional_text(
        &node,
        vocab::ct("description"),
        document.description.as_deref(),
    )?;
    graph.optional_text(&node, vocab::ct("filetype"), document.filetype.as_deref())?;
    graph.optional_text(&node, vocab::ct("format"), document.format.as_deref())
}

impl LinkSet {
    /// Encode this linkset as one deterministic `Payload triples/` RDF/XML file.
    ///
    /// Every link element must name a document of `index`. Ids resolve against
    /// `base_iri`, which must be the base the index was (or will be) written
    /// with for the document ids to match; a relative id becomes an absolute
    /// IRI under it, and an id starting with `_:` is a blank node whose label
    /// must be an XML NCName. The `filename` field is provenance and is not
    /// part of the RDF.
    ///
    /// Link types: undirected `ls:Link`; directed with exactly one from-element
    /// `ls:Directed1toNLink`; any other directed link `ls:DirectedLink`. Every
    /// link asserts `ls:hasLinkElement` for all its elements, and directed links
    /// also `ls:hasFromLinkElement` / `ls:hasToLinkElement`. Identifiers are
    /// typed `ls:StringBasedIdentifier` (with `ls:identifierField`),
    /// `ls:URIBasedIdentifier` or `ls:QueryBasedIdentifier`.
    ///
    /// Reading the result back gives this linkset with ids resolved and, as the
    /// reader's canonical order, links grouped as undirected, then
    /// `DirectedLink`, then `Directed1toNLink` (each group by id) and each
    /// link's elements sorted by id. The output is deterministic.
    ///
    /// # Errors
    ///
    /// [`IcddError::NotConformant`] for an invalid id or base, a blank node
    /// label that is not an NCName, a duplicate id, a link with fewer than two
    /// elements or with duplicate elements (same document and identifier), an
    /// undirected link with from/to ids, a directed link with no from or no to
    /// element, a duplicate or foreign from/to id, an element without a
    /// document, an element whose document is not in `index`, a query
    /// identifier lacking its language or expression, or a literal containing
    /// a C0 control character other than tab, LF and CR.
    pub fn to_rdf_xml(&self, index: &ContainerIndex, base_iri: &str) -> Result<Vec<u8>, IcddError> {
        let mut ids = Ids::new(base_iri)?;
        let documents = index
            .documents
            .iter()
            .map(|document| ids.resolve(&document.id))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut graph = Graph(Vec::new());
        for link in &self.links {
            encode_link(&mut graph, &mut ids, &documents, link)?;
        }
        crate::serialize_rdf_xml(&graph.0, linkset_options(base_iri))
    }
}

fn encode_link(
    graph: &mut Graph,
    ids: &mut Ids<'_>,
    documents: &BTreeSet<String>,
    link: &Link,
) -> Result<(), IcddError> {
    let link_id = ids.claim(&link.id, "link")?;
    let node = subject(&link_id)?;
    if link.elements.len() < 2 {
        return Err(invalid(format!(
            "link {link_id} needs at least two link elements, has {}",
            link.elements.len()
        )));
    }
    let mut contents = BTreeSet::new();
    for element in &link.elements {
        if !contents.insert(format!(
            "{:?}|{:?}",
            element.document_id, element.identifier
        )) {
            return Err(invalid(format!(
                "link {link_id} has duplicate link elements (same document and identifier)"
            )));
        }
    }
    if link.directed && (link.from.is_empty() || link.to.is_empty()) {
        return Err(invalid(format!(
            "directed link {link_id} needs at least one from and one to link element"
        )));
    }
    if !link.directed && !(link.from.is_empty() && link.to.is_empty()) {
        return Err(invalid(format!(
            "undirected link {link_id} lists from/to link elements"
        )));
    }
    let class = match (link.directed, link.from.len()) {
        (false, _) => "Link",
        (true, 1) => "Directed1toNLink",
        (true, _) => "DirectedLink",
    };
    graph.typed(&node, vocab::ls(class))?;

    let mut element_ids = BTreeSet::new();
    for element in &link.elements {
        let element_id = ids.claim(&element.id, "link element")?;
        element_ids.insert(element_id.clone());
        let element_node = subject(&element_id)?;
        graph.add(&node, vocab::ls("hasLinkElement"), object(&element_id)?)?;
        graph.typed(&element_node, vocab::ls("LinkElement"))?;
        let document = element
            .document_id
            .as_deref()
            .ok_or_else(|| invalid(format!("link element {element_id} has no document")))?;
        let document = ids.resolve(document)?;
        if !documents.contains(&document) {
            return Err(invalid(format!(
                "link element {element_id} references {document}, which is not a document of the index"
            )));
        }
        graph.add(&element_node, vocab::ls("hasDocument"), object(&document)?)?;
        if let Some(identifier) = &element.identifier {
            let identifier_id = ids.claim(&format!("{}-identifier", element.id), "identifier")?;
            let identifier_node = subject(&identifier_id)?;
            graph.add(
                &element_node,
                vocab::ls("hasIdentifier"),
                object(&identifier_id)?,
            )?;
            encode_identifier(graph, &identifier_node, identifier)?;
        }
    }

    for (predicate, members) in [
        ("hasFromLinkElement", &link.from),
        ("hasToLinkElement", &link.to),
    ] {
        let mut listed = BTreeSet::new();
        for member in members {
            let member = ids.resolve(member)?;
            if !listed.insert(member.clone()) {
                return Err(invalid(format!(
                    "link {link_id} lists {member} twice in ls:{predicate}"
                )));
            }
            if !element_ids.contains(&member) {
                return Err(invalid(format!(
                    "link {link_id} lists {member} in ls:{predicate} but it is not one of its elements"
                )));
            }
            graph.add(&node, vocab::ls(predicate), object(&member)?)?;
        }
    }
    Ok(())
}

fn encode_identifier(
    graph: &mut Graph,
    node: &NamedOrBlankNode,
    identifier: &ElementIdentifier,
) -> Result<(), IcddError> {
    match identifier {
        ElementIdentifier::String { value, field } => {
            graph.typed(node, vocab::ls("StringBasedIdentifier"))?;
            graph.text(node, vocab::ls("identifier"), value)?;
            graph.optional_text(node, vocab::ls("identifierField"), field.as_deref())
        }
        ElementIdentifier::Uri { uri } => {
            graph.typed(node, vocab::ls("URIBasedIdentifier"))?;
            graph.typed_text(node, vocab::ls("uri"), uri, xsd::ANY_URI)
        }
        ElementIdentifier::Query {
            language,
            expression,
        } => {
            graph.typed(node, vocab::ls("QueryBasedIdentifier"))?;
            let (Some(language), Some(expression)) = (language, expression) else {
                return Err(invalid(
                    "a query identifier needs both ls:queryLanguage and ls:queryExpression",
                ));
            };
            graph.text(node, vocab::ls("queryLanguage"), language)?;
            graph.text(node, vocab::ls("queryExpression"), expression)
        }
    }
}
