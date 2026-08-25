//! Parse `Index.rdf` (the Container ontology) into [`ContainerIndex`].
//!
//! The Index dataset is rooted at exactly one `ct:ContainerDescription`. Only
//! resources reached through its `ct:containsDocument` and `ct:containsLinkset`
//! edges become typed container members; extension subjects remain available in
//! the raw RDF graph API but cannot spoof ISO members by local name.

use std::collections::BTreeSet;

use super::error::IcddError;
use super::rdfgraph::RdfGraph;
use super::schema::*;
use super::vocab;

/// Parse an `Index.rdf` byte stream into the neutral container index.
pub fn parse_index(bytes: &[u8]) -> Result<ContainerIndex, IcddError> {
    let graph = RdfGraph::parse(bytes)?;
    let roots = graph.subjects_of_type_ns(vocab::CT, "ContainerDescription");
    if roots.len() != 1 {
        return Err(IcddError::NotConformant(format!(
            "Index.rdf must contain exactly one ct:ContainerDescription, found {}",
            roots.len()
        )));
    }
    let root = roots[0];

    let description = ContainerDescription {
        id: root.to_string(),
        conformance_indicator: graph.literal_ns(root, vocab::CT, "conformanceIndicator"),
        description: graph.literal_ns(root, vocab::CT, "description"),
        creation_date: graph.literal_ns(root, vocab::CT, "creationDate"),
    };

    let document_ids = contained_resources(&graph, root, "containsDocument")?;
    let mut documents = Vec::with_capacity(document_ids.len());
    for id in document_ids {
        let concrete_types = ["InternalDocument", "ExternalDocument", "FolderDocument"]
            .into_iter()
            .filter(|kind| graph.has_type_ns(&id, vocab::CT, kind))
            .collect::<Vec<_>>();
        if concrete_types.len() != 1 {
            return Err(IcddError::NotConformant(format!(
                "contained document {id} must have exactly one concrete ct:Document type"
            )));
        }
        let kind = match concrete_types[0] {
            "InternalDocument" => DocumentKind::Internal {
                filename: required_literal(&graph, &id, "filename")?,
            },
            "ExternalDocument" => DocumentKind::External {
                url: required_literal(&graph, &id, "url")?,
            },
            "FolderDocument" => DocumentKind::Folder {
                foldername: required_literal(&graph, &id, "foldername")?,
            },
            _ => unreachable!(),
        };

        let checksum = match (
            graph.literal_ns(&id, vocab::CT, "checksum"),
            graph.literal_ns(&id, vocab::CT, "checksumAlgorithm"),
        ) {
            (Some(value), Some(algorithm)) => Some(Checksum { algorithm, value }),
            (None, None) => None,
            _ => {
                return Err(IcddError::NotConformant(format!(
                    "document {id} must provide checksum and checksumAlgorithm together"
                )))
            }
        };

        documents.push(Document {
            id: id.clone(),
            kind,
            name: graph.literal_ns(&id, vocab::CT, "name"),
            description: graph.literal_ns(&id, vocab::CT, "description"),
            filetype: graph.literal_ns(&id, vocab::CT, "filetype"),
            format: graph.literal_ns(&id, vocab::CT, "format"),
            checksum,
            encrypted: graph.has_type_ns(&id, vocab::CT, "EncryptedDocument"),
            requested: graph
                .literal_ns(&id, vocab::CT, "requested")
                .is_some_and(|value| value.eq_ignore_ascii_case("true")),
        });
    }

    let linkset_ids = contained_resources(&graph, root, "containsLinkset")?;
    let mut linkset_files = Vec::with_capacity(linkset_ids.len());
    for id in linkset_ids {
        if !graph.has_type_ns(&id, vocab::CT, "Linkset") {
            return Err(IcddError::NotConformant(format!(
                "contained linkset {id} is not a ct:Linkset"
            )));
        }
        linkset_files.push(LinksetRef {
            id: id.clone(),
            filename: Some(required_literal(&graph, &id, "filename")?),
            name: graph.literal_ns(&id, vocab::CT, "name"),
        });
    }

    Ok(ContainerIndex {
        description,
        documents,
        linkset_files,
    })
}

fn contained_resources(
    graph: &RdfGraph,
    root: &str,
    predicate: &str,
) -> Result<Vec<String>, IcddError> {
    let objects = graph.objects_ns(root, vocab::CT, predicate);
    let mut seen = BTreeSet::new();
    let mut resources = Vec::with_capacity(objects.len());
    for object in objects {
        let resource = object.as_resource().ok_or_else(|| {
            IcddError::NotConformant(format!("ct:{predicate} must reference a resource"))
        })?;
        if !seen.insert(resource.to_string()) {
            return Err(IcddError::NotConformant(format!(
                "ct:{predicate} contains duplicate resource {resource}"
            )));
        }
        resources.push(resource.to_string());
    }
    Ok(resources)
}

fn required_literal(graph: &RdfGraph, subject: &str, predicate: &str) -> Result<String, IcddError> {
    graph
        .literal_ns(subject, vocab::CT, predicate)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            IcddError::NotConformant(format!("{subject} is missing non-empty ct:{predicate}"))
        })
}
