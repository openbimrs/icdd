//! Parse `Index.rdf` (the Container ontology) into [`ContainerIndex`].
//!
//! The Index dataset is a star graph rooted at a single `ct:ContainerDescription`
//! which `ct:containsDocument` / `ct:containsLinkset` its members. Documents are
//! `ct:InternalDocument` / `ct:ExternalDocument` / `ct:FolderDocument`, optionally
//! also `ct:SecuredDocument` / `ct:EncryptedDocument` (a document individual can
//! carry multiple `rdf:type`s — we fold the mix-ins into flags).

use super::error::IcddError;
use super::rdfgraph::RdfGraph;
use super::schema::*;

/// Parse an `Index.rdf` byte stream into the neutral container index.
pub fn parse_index(bytes: &[u8]) -> Result<ContainerIndex, IcddError> {
    let g = RdfGraph::parse(bytes)?;

    // The single ct:ContainerDescription.
    let cd_id = g
        .subjects_of_type("ContainerDescription")
        .into_iter()
        .next()
        .ok_or_else(|| IcddError::NotConformant("Index.rdf has no ct:ContainerDescription".into()))?
        .to_string();

    let description = ContainerDescription {
        id: cd_id.clone(),
        conformance_indicator: g.literal(&cd_id, "conformanceIndicator"),
        description: g.literal(&cd_id, "description"),
        creation_date: g.literal(&cd_id, "creationDate"),
    };

    // Documents: every subject that is a *Document type. A subject may carry
    // several rdf:types (e.g. InternalDocument + SecuredDocument), so collect
    // the union of the three concrete document classes.
    let mut doc_ids: Vec<String> = Vec::new();
    for t in ["InternalDocument", "ExternalDocument", "FolderDocument"] {
        for s in g.subjects_of_type(t) {
            if !doc_ids.iter().any(|d| d == s) {
                doc_ids.push(s.to_string());
            }
        }
    }

    let mut documents = Vec::with_capacity(doc_ids.len());
    for id in doc_ids {
        let kind = if let Some(fname) = g.literal(&id, "filename") {
            DocumentKind::Internal { filename: fname }
        } else if let Some(url) = g.literal(&id, "url") {
            DocumentKind::External { url }
        } else if let Some(folder) = g.literal(&id, "foldername") {
            DocumentKind::Folder { foldername: folder }
        } else if g.has_type(&id, "FolderDocument") {
            // FolderDocument with the foldername on a different predicate spelling.
            DocumentKind::Folder {
                foldername: String::new(),
            }
        } else {
            // Internal document whose filename we couldn't read — keep it as an
            // empty internal reference rather than dropping it silently.
            DocumentKind::Internal {
                filename: String::new(),
            }
        };

        let checksum = match (
            g.literal(&id, "checksum"),
            g.literal(&id, "checksumAlgorithm"),
        ) {
            (Some(value), Some(algorithm)) => Some(Checksum { algorithm, value }),
            _ => None,
        };

        documents.push(Document {
            id: id.clone(),
            kind,
            name: g.literal(&id, "name"),
            description: g.literal(&id, "description"),
            filetype: g.literal(&id, "filetype"),
            format: g.literal(&id, "format"),
            checksum,
            encrypted: g.has_type(&id, "EncryptedDocument"),
            requested: g.bool_true(&id, "requested"),
        });
    }

    // Linkset references.
    let mut linkset_files = Vec::new();
    for id in g.subjects_of_type("Linkset") {
        linkset_files.push(LinksetRef {
            id: id.to_string(),
            filename: g.literal(id, "filename"),
            name: g.literal(id, "name"),
        });
    }

    Ok(ContainerIndex {
        description,
        documents,
        linkset_files,
    })
}
