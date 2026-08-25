//! Canonical Poing federation extension for ICDD.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use uuid::Uuid;

use crate::rdf::{Literal, NamedNode, Triple};
use crate::rdfgraph::RdfGraph;
use crate::{IcddArchiveBuilder, IcddContainer, IcddError, RdfXmlOptions};

const CT: &str = "https://standards.iso.org/iso/21597/-1/ed-1/en/Container#";
const POING: &str = "https://poing.dev/ns/federation/icdd/1#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const FEDERATION_RDF: &str = "poing-federation.rdf";
const ONTOLOGY_RDF: &str = "poing-federation-ontology.rdf";

/// Portable federation metadata stored by the Poing ICDD extension.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoingFederationManifest {
    pub id: String,
    pub uuid: Uuid,
    pub name: String,
    pub primary_member_id: String,
    pub coordinate_reference_system: Option<String>,
    pub members: Vec<PoingFederationMember>,
}

/// One member in [`PoingFederationManifest`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoingFederationMember {
    pub id: String,
    pub uuid: Uuid,
    pub name: String,
    pub source: String,
    pub industry_domain: Option<String>,
    pub schema: Option<String>,
    pub content_hash: Option<String>,
    pub coordinate_reference_system: Option<String>,
    pub transform: [f64; 16],
}

/// One source-model document bound to a federation member.
pub enum FederationIcddPayload<'a> {
    Internal {
        member_id: &'a str,
        filename: &'a str,
        bytes: &'a [u8],
    },
    External {
        member_id: &'a str,
        url: &'a str,
    },
}

impl FederationIcddPayload<'_> {
    fn member_id(&self) -> &str {
        match self {
            Self::Internal { member_id, .. } | Self::External { member_id, .. } => member_id,
        }
    }
}

fn validate_transform(transform: &[f64; 16]) -> Result<(), IcddError> {
    if transform.iter().any(|value| !value.is_finite()) {
        return Err(invalid(
            "federation member transform must contain finite numbers",
        ));
    }
    if transform[12] != 0.0 || transform[13] != 0.0 || transform[14] != 0.0 || transform[15] != 1.0
    {
        return Err(invalid(
            "federation member transform must be an affine 4x4 matrix",
        ));
    }

    // Scale each row independently before taking the determinant. This avoids
    // overflow/underflow while preserving singularity (including proportional
    // rows with very large finite coefficients).
    let rows = [
        [transform[0], transform[1], transform[2]],
        [transform[4], transform[5], transform[6]],
        [transform[8], transform[9], transform[10]],
    ];
    let scales = rows.map(|row| row.into_iter().map(f64::abs).fold(0.0, f64::max));
    if scales.contains(&0.0) {
        return Err(invalid("federation member transform must be invertible"));
    }
    let normalized = [
        rows[0].map(|value| value / scales[0]),
        rows[1].map(|value| value / scales[1]),
        rows[2].map(|value| value / scales[2]),
    ];
    let [[a, b, c], [d, e, f], [g, h, i]] = normalized;
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    let log_abs_determinant =
        determinant.abs().ln() + scales[0].ln() + scales[1].ln() + scales[2].ln();
    if !determinant.is_finite() || determinant == 0.0 || !log_abs_determinant.is_finite() {
        return Err(invalid("federation member transform must be invertible"));
    }

    // A nonzero determinant is not sufficient when the inverse would overflow
    // `f64` (for example, a `1e-320` axis scale). `A = D * N`, where `D`
    // contains the row scales above, so `A^-1 = N^-1 * D^-1`. Check the
    // adjugate in log space to avoid overflowing while proving every inverse
    // component remains representable. Zero cofactors are exact zeroes.
    let inverse_cofactors = [
        [e * i - f * h, c * h - b * i, b * f - c * e],
        [f * g - d * i, a * i - c * g, c * d - a * f],
        [d * h - e * g, b * g - a * h, a * e - b * d],
    ];
    let log_abs_normalized_determinant = determinant.abs().ln();
    let log_max = f64::MAX.ln();
    for row in inverse_cofactors {
        for (column, cofactor) in row.into_iter().enumerate() {
            if cofactor != 0.0
                && cofactor.abs().ln() - log_abs_normalized_determinant - scales[column].ln()
                    > log_max
            {
                return Err(invalid("federation member transform must be invertible"));
            }
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &PoingFederationManifest) -> Result<(), IcddError> {
    if manifest.id.trim().is_empty() || manifest.name.trim().is_empty() {
        return Err(invalid("federation id and name must not be empty"));
    }
    if manifest.members.is_empty() {
        return Err(invalid("federation must contain at least one member"));
    }
    let mut ids = BTreeSet::new();
    let mut uuids = BTreeSet::new();
    for member in &manifest.members {
        if member.id.trim().is_empty()
            || member.name.trim().is_empty()
            || member.source.trim().is_empty()
        {
            return Err(invalid(
                "federation member identity fields must not be empty",
            ));
        }
        if !ids.insert(member.id.as_str()) || !uuids.insert(member.uuid) {
            return Err(invalid("federation member ids and UUIDs must be unique"));
        }
        validate_transform(&member.transform)?;
    }
    if !ids.contains(manifest.primary_member_id.as_str()) {
        return Err(invalid("primary federation member does not exist"));
    }
    Ok(())
}

fn validate_bindings<'a>(
    manifest: &PoingFederationManifest,
    payloads: &'a [FederationIcddPayload<'a>],
) -> Result<BTreeMap<&'a str, &'a FederationIcddPayload<'a>>, IcddError> {
    let mut result = BTreeMap::new();
    for payload in payloads {
        if result.insert(payload.member_id(), payload).is_some() {
            return Err(invalid(&format!(
                "duplicate payload binding for {}",
                payload.member_id()
            )));
        }
    }
    for member in &manifest.members {
        let payload = result
            .get(member.id.as_str())
            .ok_or_else(|| invalid(&format!("missing payload binding for {}", member.id)))?;
        let bound_source = match payload {
            FederationIcddPayload::Internal { filename, .. } => *filename,
            FederationIcddPayload::External { url, .. } => *url,
        };
        if member.source != bound_source {
            return Err(invalid(&format!(
                "federation member {} source {:?} does not match bound source {:?}",
                member.id, member.source, bound_source
            )));
        }
    }
    if result.len() != manifest.members.len() {
        return Err(invalid(
            "payload binding references an unknown federation member",
        ));
    }
    Ok(result)
}

fn node(iri: impl Into<String>) -> Result<NamedNode, IcddError> {
    NamedNode::new(iri.into()).map_err(|error| IcddError::Rdf(error.to_string()))
}

fn resource(subject: &NamedNode, predicate: &str, object: &str) -> Result<Triple, IcddError> {
    Ok(Triple::new(
        subject.clone(),
        node(predicate)?,
        node(object)?,
    ))
}

fn literal(
    subject: &NamedNode,
    predicate: &str,
    value: impl Into<String>,
) -> Result<Triple, IcddError> {
    Ok(Triple::new(
        subject.clone(),
        node(predicate)?,
        Literal::new_simple_literal(value.into()),
    ))
}

fn index_graph(
    manifest: &PoingFederationManifest,
    bindings: &BTreeMap<&str, &FederationIcddPayload<'_>>,
) -> Result<Vec<Triple>, IcddError> {
    let base = format!("https://poing.dev/icdd/{}/", manifest.uuid);
    let container = node(format!("{base}container-{}", manifest.uuid))?;
    let metadata = node(format!("{base}federation-metadata"))?;
    let mut triples = vec![
        resource(&container, RDF_TYPE, &format!("{CT}ContainerDescription"))?,
        literal(
            &container,
            &format!("{CT}conformanceIndicator"),
            "ICDD-Part1-Container",
        )?,
        literal(&container, &format!("{CT}description"), &manifest.name)?,
        resource(
            &container,
            &format!("{CT}containsDocument"),
            metadata.as_str(),
        )?,
        resource(&metadata, RDF_TYPE, &format!("{CT}InternalDocument"))?,
        literal(&metadata, &format!("{CT}name"), "Poing federation manifest")?,
        literal(&metadata, &format!("{CT}filename"), FEDERATION_RDF)?,
        literal(&metadata, &format!("{CT}filetype"), "rdf")?,
        literal(&metadata, &format!("{CT}format"), "application/rdf+xml")?,
    ];
    for member in &manifest.members {
        let member_node = node(format!("{base}member-{}", member.uuid))?;
        triples.push(resource(
            &container,
            &format!("{CT}containsDocument"),
            member_node.as_str(),
        )?);
        match bindings[member.id.as_str()] {
            FederationIcddPayload::Internal { filename, .. } => {
                triples.push(resource(
                    &member_node,
                    RDF_TYPE,
                    &format!("{CT}InternalDocument"),
                )?);
                triples.push(literal(&member_node, &format!("{CT}filename"), *filename)?);
            }
            FederationIcddPayload::External { url, .. } => {
                triples.push(resource(
                    &member_node,
                    RDF_TYPE,
                    &format!("{CT}ExternalDocument"),
                )?);
                triples.push(literal(&member_node, &format!("{CT}url"), *url)?);
            }
        }
        triples.push(literal(&member_node, &format!("{CT}name"), &member.name)?);
    }
    Ok(triples)
}

fn federation_graph(manifest: &PoingFederationManifest) -> Result<Vec<Triple>, IcddError> {
    let base = format!("https://poing.dev/icdd/{}/federation/", manifest.uuid);
    let root = node(format!("{base}federation-{}", manifest.uuid))?;
    let mut triples = vec![
        resource(&root, RDF_TYPE, &format!("{POING}Federation"))?,
        literal(&root, &format!("{POING}id"), &manifest.id)?,
        literal(&root, &format!("{POING}uuid"), manifest.uuid.to_string())?,
        literal(&root, &format!("{POING}name"), &manifest.name)?,
        literal(
            &root,
            &format!("{POING}primaryMemberId"),
            &manifest.primary_member_id,
        )?,
    ];
    if let Some(crs) = &manifest.coordinate_reference_system {
        triples.push(literal(&root, &format!("{POING}crs"), crs)?);
    }
    for (order, member) in manifest.members.iter().enumerate() {
        let member_node = node(format!("{base}member-{}", member.uuid))?;
        triples.extend([
            resource(&member_node, RDF_TYPE, &format!("{POING}Member"))?,
            literal(&member_node, &format!("{POING}order"), order.to_string())?,
            literal(&member_node, &format!("{POING}id"), &member.id)?,
            literal(
                &member_node,
                &format!("{POING}uuid"),
                member.uuid.to_string(),
            )?,
            literal(&member_node, &format!("{POING}name"), &member.name)?,
            literal(&member_node, &format!("{POING}source"), &member.source)?,
            literal(
                &member_node,
                &format!("{POING}transform"),
                member
                    .transform
                    .iter()
                    .map(f64::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            )?,
        ]);
        for (predicate, value) in [
            ("industryDomain", member.industry_domain.as_deref()),
            ("schema", member.schema.as_deref()),
            ("contentHash", member.content_hash.as_deref()),
            ("crs", member.coordinate_reference_system.as_deref()),
        ] {
            if let Some(value) = value {
                triples.push(literal(
                    &member_node,
                    &format!("{POING}{predicate}"),
                    value,
                )?);
            }
        }
    }
    Ok(triples)
}

fn ontology_graph() -> Result<Vec<Triple>, IcddError> {
    let ontology = node(POING.trim_end_matches('#'))?;
    let federation = node(format!("{POING}Federation"))?;
    let member = node(format!("{POING}Member"))?;
    Ok(vec![
        resource(&ontology, RDF_TYPE, OWL_ONTOLOGY)?,
        resource(&federation, RDF_TYPE, RDFS_CLASS)?,
        resource(&member, RDF_TYPE, RDFS_CLASS)?,
    ])
}

fn encode(triples: &[Triple], base: &str) -> Result<Vec<u8>, IcddError> {
    crate::serialize_rdf_xml(
        triples,
        RdfXmlOptions::new()
            .with_base_iri(base)
            .with_prefix("ct", CT)
            .with_prefix("poing", POING),
    )
}

/// Emit a deterministic ISO 21597-1 container plus a Poing federation graph.
pub fn write_poing_federation_icdd(
    manifest: &PoingFederationManifest,
    payloads: &[FederationIcddPayload<'_>],
) -> Result<Vec<u8>, IcddError> {
    validate_manifest(manifest)?;
    let bindings = validate_bindings(manifest, payloads)?;
    let base = format!("https://poing.dev/icdd/{}/", manifest.uuid);
    let index = encode(&index_graph(manifest, &bindings)?, &base)?;
    let federation_base = format!("{base}federation/");
    let federation = encode(&federation_graph(manifest)?, &federation_base)?;
    let ontology = encode(&ontology_graph()?, POING.trim_end_matches('#'))?;

    let mut archive = IcddArchiveBuilder::new(index)?
        .add_ontology_resource(ONTOLOGY_RDF, ontology)?
        .add_payload(FEDERATION_RDF, federation)?;
    for payload in payloads {
        if let FederationIcddPayload::Internal {
            filename, bytes, ..
        } = payload
        {
            archive = archive.add_payload(*filename, *bytes)?;
        }
    }
    archive.finish()
}

/// Parse the portable Poing federation extension from an ICDD container.
pub fn parse_poing_federation_icdd(bytes: &[u8]) -> Result<PoingFederationManifest, IcddError> {
    let mut container = IcddContainer::open(std::io::Cursor::new(bytes))?;
    let document = container
        .index()
        .documents
        .iter()
        .find(|document| document.internal_path() == Some(FEDERATION_RDF))
        .cloned()
        .ok_or_else(|| invalid("ICDD has no Poing federation metadata document"))?;
    let source_documents = container.index().documents.clone();
    let rdf = container.payload_bytes(&document)?;
    let graph = RdfGraph::parse(rdf.as_slice())?;
    let roots = graph.subjects_of_type_ns(POING, "Federation");
    if roots.len() != 1 {
        return Err(invalid(&format!(
            "Poing federation RDF must contain exactly one Federation subject, found {}",
            roots.len()
        )));
    }
    let root = roots[0];
    let mut members = graph
        .subjects_of_type_ns(POING, "Member")
        .into_iter()
        .map(|subject| parse_member(&graph, subject))
        .collect::<Result<Vec<_>, _>>()?;
    members.sort_by_key(|(order, _)| *order);
    for (expected, (actual, _)) in members.iter().enumerate() {
        if expected != *actual {
            return Err(invalid("Poing member order must be contiguous from zero"));
        }
    }
    let manifest = PoingFederationManifest {
        id: required(&graph, root, "id")?,
        uuid: parse_uuid(&required(&graph, root, "uuid")?)?,
        name: required(&graph, root, "name")?,
        primary_member_id: required(&graph, root, "primaryMemberId")?,
        coordinate_reference_system: graph.literal_ns(root, POING, "crs"),
        members: members.into_iter().map(|(_, member)| member).collect(),
    };
    validate_manifest(&manifest)?;
    for member in &manifest.members {
        let matching = source_documents
            .iter()
            .filter(|candidate| match &candidate.kind {
                crate::DocumentKind::Internal { filename } => filename == &member.source,
                crate::DocumentKind::External { url } => url == &member.source,
                crate::DocumentKind::Folder { .. } => false,
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(invalid(&format!(
                "federation member {} source {:?} must identify exactly one Index.rdf document",
                member.id, member.source
            )));
        }
        let source_document = matching[0];
        if source_document.id == document.id || source_document.requested {
            return Err(invalid(&format!(
                "federation member {} source {:?} is not an available model document",
                member.id, member.source
            )));
        }
        if let crate::DocumentKind::Internal { filename } = &source_document.kind {
            if !container.contains_internal_payload(filename) {
                return Err(invalid(&format!(
                    "federation member {} has unavailable internal source {:?}",
                    member.id, member.source
                )));
            }
        }
    }
    Ok(manifest)
}

fn parse_member(
    graph: &RdfGraph,
    subject: &str,
) -> Result<(usize, PoingFederationMember), IcddError> {
    let order = required(graph, subject, "order")?
        .parse::<usize>()
        .map_err(|_| invalid("Poing member order is not an unsigned integer"))?;
    Ok((
        order,
        PoingFederationMember {
            id: required(graph, subject, "id")?,
            uuid: parse_uuid(&required(graph, subject, "uuid")?)?,
            name: required(graph, subject, "name")?,
            source: required(graph, subject, "source")?,
            industry_domain: graph.literal_ns(subject, POING, "industryDomain"),
            schema: graph.literal_ns(subject, POING, "schema"),
            content_hash: graph.literal_ns(subject, POING, "contentHash"),
            coordinate_reference_system: graph.literal_ns(subject, POING, "crs"),
            transform: parse_transform(&required(graph, subject, "transform")?)?,
        },
    ))
}

fn parse_transform(value: &str) -> Result<[f64; 16], IcddError> {
    let values = value
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("Poing member transform contains a non-number"))?;
    let transform: [f64; 16] = values
        .try_into()
        .map_err(|_| invalid("Poing member transform must contain exactly 16 numbers"))?;
    validate_transform(&transform)?;
    Ok(transform)
}

fn parse_uuid(value: &str) -> Result<Uuid, IcddError> {
    Uuid::parse_str(value).map_err(|_| invalid("Poing UUID is not a valid UUID"))
}

fn required(graph: &RdfGraph, subject: &str, predicate: &str) -> Result<String, IcddError> {
    graph
        .literal_ns(subject, POING, predicate)
        .ok_or_else(|| invalid(&format!("Poing federation RDF is missing {predicate}")))
}

fn invalid(message: &str) -> IcddError {
    IcddError::NotConformant(message.to_owned())
}
