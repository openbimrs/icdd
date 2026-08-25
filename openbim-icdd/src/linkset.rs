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

/// Parse one link dataset file. `filename` is recorded on the returned
/// [`LinkSet`] for provenance.
pub fn parse_linkset(filename: &str, bytes: &[u8]) -> Result<LinkSet, IcddError> {
    let g = RdfGraph::parse(bytes)?;

    let mut links = Vec::new();
    for link_id in g.subjects_of_type("Link") {
        let from: Vec<String> = g
            .objects_local(link_id, "hasFromLinkElement")
            .iter()
            .filter_map(|o| o.as_resource().map(str::to_string))
            .collect();
        let to: Vec<String> = g
            .objects_local(link_id, "hasToLinkElement")
            .iter()
            .filter_map(|o| o.as_resource().map(str::to_string))
            .collect();

        // All elements: hasLinkElement is the super-property, but real files
        // often assert ONLY the from/to sub-properties (which are sub-properties
        // of hasLinkElement but not materialized), so union all three.
        let mut elem_ids: Vec<String> = g
            .objects_local(link_id, "hasLinkElement")
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
            || g.has_type(link_id, "DirectedLink")
            || g.has_type(link_id, "DirectedBinaryLink")
            || g.has_type(link_id, "Directed1toNLink");

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
    let document_id = g.resource(eid, "hasDocument");
    let identifier = g
        .resource(eid, "hasIdentifier")
        .and_then(|id_node| parse_identifier(g, &id_node));
    LinkElement {
        id: eid.to_string(),
        document_id,
        identifier,
    }
}

fn parse_identifier(g: &RdfGraph, id_node: &str) -> Option<ElementIdentifier> {
    // Dispatch on the identifier's rdf:type; tolerate the value being present
    // even if the type triple is missing (fall back by which predicate exists).
    if g.has_type(id_node, "StringBasedIdentifier") || has_local(g, id_node, "identifier") {
        return Some(ElementIdentifier::String {
            value: g.literal(id_node, "identifier").unwrap_or_default(),
            field: g.literal(id_node, "identifierField"),
        });
    }
    if g.has_type(id_node, "URIBasedIdentifier") || has_local(g, id_node, "uri") {
        return Some(ElementIdentifier::Uri {
            uri: g.literal(id_node, "uri").unwrap_or_default(),
        });
    }
    if g.has_type(id_node, "QueryBasedIdentifier")
        || has_local(g, id_node, "queryExpression")
        || has_local(g, id_node, "queryLanguage")
    {
        return Some(ElementIdentifier::Query {
            language: g.literal(id_node, "queryLanguage"),
            expression: g.literal(id_node, "queryExpression"),
        });
    }
    None
}

fn has_local(g: &RdfGraph, subject: &str, pred_local: &str) -> bool {
    !g.objects_local(subject, pred_local).is_empty()
}
