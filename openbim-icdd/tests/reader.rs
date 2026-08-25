use std::io::{Cursor, Write};

use openbim_icdd::{DocumentKind, ElementIdentifier, IcddContainer};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const INDEX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#">
  <ct:ContainerDescription rdf:about="https://example.test/container">
    <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
    <ct:description>Synthetic container</ct:description>
    <ct:containsDocument rdf:resource="https://example.test/document/model"/>
    <ct:containsLinkset rdf:resource="https://example.test/linkset/main"/>
  </ct:ContainerDescription>
  <ct:InternalDocument rdf:about="https://example.test/document/model">
    <ct:filename>model.ifc</ct:filename>
    <ct:name>Model</ct:name>
    <ct:filetype>ifc</ct:filetype>
    <ct:format>application/x-step</ct:format>
  </ct:InternalDocument>
  <ct:Linkset rdf:about="https://example.test/linkset/main">
    <ct:filename>links.rdf</ct:filename>
    <ct:name>Main links</ct:name>
  </ct:Linkset>
</rdf:RDF>
"#;

const LINKSET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ls="https://standards.iso.org/iso/21597/-1/ed-1/en/Linkset#">
  <ls:Link rdf:about="https://example.test/link/main">
    <ls:hasLinkElement rdf:resource="https://example.test/element/model"/>
  </ls:Link>
  <ls:LinkElement rdf:about="https://example.test/element/model">
    <ls:hasDocument rdf:resource="https://example.test/document/model"/>
    <ls:hasIdentifier rdf:resource="https://example.test/identifier/guid"/>
  </ls:LinkElement>
  <ls:StringBasedIdentifier rdf:about="https://example.test/identifier/guid">
    <ls:identifier>3vB2YO$MX4xv5uCqZZG05x</ls:identifier>
    <ls:identifierField>GlobalId</ls:identifierField>
  </ls:StringBasedIdentifier>
</rdf:RDF>
"#;

fn synthetic_icdd() -> Vec<u8> {
    archive(INDEX, Some(LINKSET))
}

fn archive(index: &str, linkset: Option<&str>) -> Vec<u8> {
    archive_with_payload(index, linkset, true)
}

fn archive_with_payload(index: &str, linkset: Option<&str>, include_payload: bool) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    for directory in [
        "Ontology resources/",
        "Payload documents/",
        "Payload triples/",
    ] {
        zip.add_directory(directory, options).unwrap();
    }
    zip.start_file("Index.rdf", options).unwrap();
    zip.write_all(index.as_bytes()).unwrap();
    if include_payload {
        zip.start_file("Payload documents/model.ifc", options)
            .unwrap();
        zip.write_all(b"ISO-10303-21;\nEND-ISO-10303-21;\n")
            .unwrap();
    }
    if let Some(linkset) = linkset {
        zip.start_file("Payload triples/links.rdf", options)
            .unwrap();
        zip.write_all(linkset.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[test]
fn reads_index_linksets_and_payloads_through_the_canonical_crate() {
    let mut container = IcddContainer::open_bytes(synthetic_icdd()).unwrap();

    assert!(container.conformance_issues().is_empty());
    assert_eq!(container.index().documents.len(), 1);
    let document = container.index().documents[0].clone();
    assert!(matches!(document.kind, DocumentKind::Internal { .. }));
    assert!(document.is_ifc());
    assert_eq!(
        container.payload_bytes(&document).unwrap(),
        b"ISO-10303-21;\nEND-ISO-10303-21;\n"
    );
    let mut streamed = Vec::new();
    assert_eq!(
        container.copy_payload_to(&document, &mut streamed).unwrap(),
        32
    );
    assert_eq!(streamed, b"ISO-10303-21;\nEND-ISO-10303-21;\n");

    let links = &container.linksets()[0].links;
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].elements.len(), 1);
    assert!(matches!(
        links[0].elements[0].identifier,
        Some(ElementIdentifier::String { ref value, .. })
            if value == "3vB2YO$MX4xv5uCqZZG05x"
    ));
}

#[test]
fn subtype_only_directed_links_are_preserved() {
    let subtype = LINKSET
        .replace("<ls:Link rdf:about", "<ls:DirectedBinaryLink rdf:about")
        .replace("</ls:Link>", "</ls:DirectedBinaryLink>");
    let container = IcddContainer::open_bytes(archive(INDEX, Some(&subtype))).unwrap();
    assert_eq!(container.linksets()[0].links.len(), 1);
    assert!(container.linksets()[0].links[0].directed);
}

#[test]
fn uncontained_and_spoof_namespaced_members_are_not_typed() {
    let uncontained = INDEX.replace(
        "    <ct:containsDocument rdf:resource=\"https://example.test/document/model\"/>\n",
        "",
    );
    let container = IcddContainer::open_bytes(archive(&uncontained, Some(LINKSET))).unwrap();
    assert!(container.index().documents.is_empty());

    let spoofed = INDEX.replace(
        "https://standards.iso.org/iso/21597/-1/ed-1/en/Container#",
        "https://vendor.example/spoof#",
    );
    assert!(IcddContainer::open_bytes(archive(&spoofed, Some(LINKSET))).is_err());
}

#[test]
fn declared_linksets_must_exist_and_undeclared_ones_are_not_substituted() {
    assert!(IcddContainer::open_bytes(archive(INDEX, None)).is_err());
    let undeclared = INDEX
        .replace(
            "    <ct:containsLinkset rdf:resource=\"https://example.test/linkset/main\"/>\n",
            "",
        )
        .replace(
            "  <ct:Linkset rdf:about=\"https://example.test/linkset/main\">\n    <ct:filename>links.rdf</ct:filename>\n    <ct:name>Main links</ct:name>\n  </ct:Linkset>\n",
            "",
        );
    let container = IcddContainer::open_bytes(archive(&undeclared, Some(LINKSET))).unwrap();
    assert!(container.linksets().is_empty());
}

#[test]
fn document_order_is_reproducible() {
    let first: Vec<_> = IcddContainer::open_bytes(synthetic_icdd())
        .unwrap()
        .index()
        .documents
        .iter()
        .map(|document| document.id.clone())
        .collect();
    for _ in 0..8 {
        let next: Vec<_> = IcddContainer::open_bytes(synthetic_icdd())
            .unwrap()
            .index()
            .documents
            .iter()
            .map(|document| document.id.clone())
            .collect();
        assert_eq!(first, next);
    }
}

#[test]
fn conformance_reports_missing_declared_internal_documents() {
    let container = IcddContainer::open_bytes(archive_with_payload(INDEX, Some(LINKSET), false))
        .expect("missing payload is reportable rather than an open error");
    assert!(container
        .conformance_issues()
        .iter()
        .any(|issue| issue.contains("missing declared payload document: model.ifc")));
}
