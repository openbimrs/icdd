use openbim_icdd::{IcddArchiveBuilder, IcddContainer};

const INDEX: &str = r#"<rdf:RDF
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#">
  <ct:ContainerDescription rdf:about="urn:test:container">
    <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
    <ct:containsDocument rdf:resource="urn:test:document"/>
    <ct:containsLinkset rdf:resource="urn:test:linkset"/>
  </ct:ContainerDescription>
  <ct:InternalDocument rdf:about="urn:test:document">
    <ct:filename>model.ifc</ct:filename>
    <ct:filetype>ifc</ct:filetype>
  </ct:InternalDocument>
  <ct:Linkset rdf:about="urn:test:linkset">
    <ct:filename>links.rdf</ct:filename>
  </ct:Linkset>
</rdf:RDF>"#;

const LINKSET: &str = r#"<rdf:RDF
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:ls="https://standards.iso.org/iso/21597/-1/ed-1/en/Linkset#">
  <ls:Link rdf:about="urn:test:link" />
</rdf:RDF>"#;

fn build() -> Vec<u8> {
    IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("model.ifc", b"ISO-10303-21;\n")
        .unwrap()
        .add_linkset("links.rdf", LINKSET.as_bytes())
        .unwrap()
        .finish()
        .unwrap()
}

#[test]
fn writes_reopenable_deterministic_icdd_archives() {
    let first = build();
    let second = build();
    assert_eq!(first, second);

    let mut container = IcddContainer::open_bytes(first).unwrap();
    assert_eq!(container.index().documents.len(), 1);
    assert_eq!(container.linksets().len(), 1);
    assert_eq!(
        container
            .payload_bytes(&container.index().documents[0].clone())
            .unwrap(),
        b"ISO-10303-21;\n"
    );
}

#[test]
fn builder_rejects_unsafe_or_duplicate_paths() {
    assert!(IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("../escape.ifc", b"x")
        .is_err());
    assert!(IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("nested\\model.ifc", b"x")
        .is_err());
    assert!(IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("/model.ifc", b"x")
        .is_err());

    let duplicate = IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("model.ifc", b"one")
        .unwrap()
        .add_payload("MODEL.ifc", b"two");
    assert!(duplicate.is_err());
}

#[test]
fn builder_rejects_malformed_rdf_before_writing() {
    assert!(IcddArchiveBuilder::new(b"not RDF/XML").is_err());
    assert!(IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_linkset("links.rdf", b"not RDF/XML")
        .is_err());
}

#[test]
fn builder_rejects_missing_declared_internal_documents() {
    let error = IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_linkset("links.rdf", LINKSET.as_bytes())
        .unwrap()
        .finish()
        .expect_err("declared model.ifc is missing");
    assert!(error.to_string().contains("model.ifc"));
}

#[test]
fn builder_rejects_missing_declared_linksets() {
    let error = IcddArchiveBuilder::new(INDEX.as_bytes())
        .unwrap()
        .add_payload("model.ifc", b"ISO-10303-21;")
        .unwrap()
        .finish()
        .expect_err("declared links.rdf is missing");
    assert!(error.to_string().contains("links.rdf"));
}
