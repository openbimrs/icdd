use std::io::{Cursor, Write};
use std::path::PathBuf;

use openbim_icdd::IcddContainer;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn zip_with_index(index: &[u8], extra_name: Option<&str>, extra: &[u8]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("Index.rdf", options).unwrap();
    zip.write_all(index).unwrap();
    if let Some(name) = extra_name {
        zip.start_file(name, options).unwrap();
        zip.write_all(extra).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn malicious_icdd() -> Vec<u8> {
    let index = r##"<rdf:RDF
      xml:base="https://example.test/"
      xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
      xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#">
      <ct:ContainerDescription rdf:about="#container">
        <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
      </ct:ContainerDescription>
      <ct:InternalDocument rdf:about="#doc">
        <ct:filename>../../escape.ifc</ct:filename>
        <ct:filetype>ifc</ct:filetype>
      </ct:InternalDocument>
    </rdf:RDF>"##;
    zip_with_index(
        index.as_bytes(),
        Some("Payload documents/../../escape.ifc"),
        b"not allowed",
    )
}

#[test]
fn extraction_rejects_parent_directory_payload_paths() {
    let mut container = IcddContainer::open_bytes(malicious_icdd()).unwrap();
    let root =
        std::env::temp_dir().join(format!("openbim-icdd-path-safety-{}", std::process::id()));
    let extraction = root.join("nested/output");
    let escaped: PathBuf = root.join("escape.ifc");
    let _ = std::fs::remove_dir_all(&root);

    let result = container.extract_payloads(&extraction);
    assert!(result.is_err(), "unsafe archive path must be rejected");
    assert!(!escaped.exists(), "payload escaped extraction root");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compressed_rdf_metadata_cannot_expand_without_a_limit() {
    let huge_literal = "x".repeat(17 * 1024 * 1024);
    let index = format!(
        r##"<rdf:RDF xml:base="https://example.test/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#" xmlns:v="https://vendor.example/#"><ct:ContainerDescription rdf:about="#container"><ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator><v:blob>{huge_literal}</v:blob></ct:ContainerDescription></rdf:RDF>"##
    );
    let error = match IcddContainer::open_bytes(zip_with_index(index.as_bytes(), None, &[])) {
        Ok(_) => panic!("oversized metadata must fail"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("limit"),
        "unexpected error: {error}"
    );
}
