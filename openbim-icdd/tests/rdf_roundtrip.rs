use std::collections::BTreeSet;

use openbim_icdd::{parse_rdf_xml, serialize_rdf_xml, RdfXmlOptions};

const RDF: &str = r##"<rdf:RDF
  xml:base="https://example.test/base/"
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#"
  xmlns:vendor="https://vendor.example/ns#">
  <ct:ContainerDescription rdf:about="#container">
    <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
    <vendor:unknown xml:lang="de">erhalten</vendor:unknown>
    <vendor:resource rdf:resource="https://vendor.example/object"/>
  </ct:ContainerDescription>
</rdf:RDF>"##;

fn canonical(triples: &[oxrdf::Triple]) -> BTreeSet<String> {
    triples.iter().map(ToString::to_string).collect()
}

#[test]
fn unknown_rdf_triples_survive_semantic_round_trip() {
    let parsed = parse_rdf_xml(RDF.as_bytes()).unwrap();
    let encoded = serialize_rdf_xml(
        &parsed,
        RdfXmlOptions::new()
            .with_base_iri("https://example.test/base/")
            .with_prefix(
                "ct",
                "https://standards.iso.org/iso/21597/-1/ed-1/en/Container#",
            )
            .with_prefix("vendor", "https://vendor.example/ns#"),
    )
    .unwrap();
    let reparsed = parse_rdf_xml(encoded.as_slice()).unwrap();
    assert_eq!(canonical(&parsed), canonical(&reparsed));
}

#[test]
fn serializer_output_is_deterministic_for_the_same_graph() {
    let parsed = parse_rdf_xml(RDF.as_bytes()).unwrap();
    let options = RdfXmlOptions::new();
    assert_eq!(
        serialize_rdf_xml(&parsed, options.clone()).unwrap(),
        serialize_rdf_xml(&parsed, options).unwrap()
    );
}
