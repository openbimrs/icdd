//! Parse a link dataset (`Payload triples/*.rdf`, the Linkset ontology) into
//! [`Vec<Link>`].
//!
//! Structure: `ls:Link` --`ls:hasLinkElement`--> `ls:LinkElement`
//! --`ls:hasDocument`--> `ct:Document`, and optionally
//! --`ls:hasIdentifier`--> `ls:Identifier` (String / URI / Query based).
//! Directed links additionally split their elements into
//! `ls:hasFromLinkElement` / `ls:hasToLinkElement` (sub-properties of
//! `hasLinkElement`). `DirectedLink` / `BinaryLink` / `Directed1toNLink` etc.
//! are all `ls:Link` subtypes, so we detect a directed link by the presence of
//! from/to properties or a Directed* rdf:type.

use super::error::IcddError;
use super::rdfgraph::RdfGraph;
use super::schema::*;
use super::vocab;

/// Parse one link dataset file. `filename` is recorded on the returned
/// [`LinkSet`] for provenance.
pub fn parse_linkset(filename: &str, bytes: &[u8]) -> Result<LinkSet, IcddError> {
    let g = RdfGraph::parse(bytes)?;

    let mut link_ids = Vec::new();
    for link_type in [
        "Link",
        "BinaryLink",
        "DirectedLink",
        "DirectedBinaryLink",
        "Directed1toNLink",
        "Directed1ToNLink",
    ] {
        for id in g.subjects_of_type_ns(vocab::LS, link_type) {
            if !link_ids.contains(&id) {
                link_ids.push(id);
            }
        }
    }

    let mut links = Vec::new();
    for link_id in link_ids {
        let from: Vec<String> = g
            .objects_ns(link_id, vocab::LS, "hasFromLinkElement")
            .iter()
            .filter_map(|o| o.as_resource().map(str::to_string))
            .collect();
        let to: Vec<String> = g
            .objects_ns(link_id, vocab::LS, "hasToLinkElement")
            .iter()
            .filter_map(|o| o.as_resource().map(str::to_string))
            .collect();

        // All elements: hasLinkElement is the super-property, but real files
        // often assert ONLY the from/to sub-properties (which are sub-properties
        // of hasLinkElement but not materialized), so union all three.
        let mut elem_ids: Vec<String> = g
            .objects_ns(link_id, vocab::LS, "hasLinkElement")
            .iter()
            .filter_map(|o| o.as_resource().map(str::to_string))
            .collect();
        for id in from.iter().chain(to.iter()) {
            if !elem_ids.contains(id) {
                elem_ids.push(id.clone());
            }
        }

        let directed = !from.is_empty()
            || !to.is_empty()
            || [
                "DirectedLink",
                "DirectedBinaryLink",
                "Directed1toNLink",
                "Directed1ToNLink",
            ]
            .iter()
            .any(|kind| g.has_type_ns(link_id, vocab::LS, kind));

        let elements = elem_ids
            .iter()
            .map(|eid| parse_link_element(&g, eid))
            .collect();

        links.push(Link {
            id: link_id.to_string(),
            directed,
            elements,
            from,
            to,
        });
    }

    Ok(LinkSet {
        filename: filename.to_string(),
        links,
    })
}

fn parse_link_element(g: &RdfGraph, eid: &str) -> LinkElement {
    let document_id = g.resource_ns(eid, vocab::LS, "hasDocument");
    let identifier = g
        .resource_ns(eid, vocab::LS, "hasIdentifier")
        .and_then(|id_node| parse_identifier(g, &id_node));
    LinkElement {
        id: eid.to_string(),
        document_id,
        identifier,
    }
}

fn parse_identifier(g: &RdfGraph, id_node: &str) -> Option<ElementIdentifier> {
    if g.has_type_ns(id_node, vocab::LS, "StringBasedIdentifier")
        || has_ls(g, id_node, "identifier")
    {
        return Some(ElementIdentifier::String {
            value: g
                .literal_ns(id_node, vocab::LS, "identifier")
                .unwrap_or_default(),
            field: g.literal_ns(id_node, vocab::LS, "identifierField"),
        });
    }
    if g.has_type_ns(id_node, vocab::LS, "URIBasedIdentifier") || has_ls(g, id_node, "uri") {
        return Some(ElementIdentifier::Uri {
            uri: g.literal_ns(id_node, vocab::LS, "uri").unwrap_or_default(),
        });
    }
    if g.has_type_ns(id_node, vocab::LS, "QueryBasedIdentifier")
        || has_ls(g, id_node, "queryExpression")
        || has_ls(g, id_node, "queryLanguage")
    {
        return Some(ElementIdentifier::Query {
            language: g.literal_ns(id_node, vocab::LS, "queryLanguage"),
            expression: g.literal_ns(id_node, vocab::LS, "queryExpression"),
        });
    }
    None
}

fn has_ls(g: &RdfGraph, subject: &str, pred_local: &str) -> bool {
    !g.objects_ns(subject, vocab::LS, pred_local).is_empty()
}
