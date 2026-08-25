use std::io::{Cursor, Write};

use openbim_icdd::IcddContainer;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const MINIMAL_INDEX: &str = r##"<rdf:RDF
  xml:base="https://example.test/"
  xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#">
  <ct:ContainerDescription rdf:about="#container">
    <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
  </ct:ContainerDescription>
</rdf:RDF>"##;

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

fn icdd_with_filename(filename: &str) -> Vec<u8> {
    icdd_with_declared_and_entry(filename, filename)
}

fn icdd_with_declared_and_entry(declared: &str, entry: &str) -> Vec<u8> {
    let index = r##"<rdf:RDF
      xml:base="https://example.test/"
      xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
      xmlns:ct="https://standards.iso.org/iso/21597/-1/ed-1/en/Container#">
      <ct:ContainerDescription rdf:about="#container">
        <ct:conformanceIndicator>ICDD-Part1-Container</ct:conformanceIndicator>
        <ct:containsDocument rdf:resource="#doc"/>
      </ct:ContainerDescription>
      <ct:InternalDocument rdf:about="#doc">
        <ct:filename>__FILENAME__</ct:filename>
        <ct:filetype>ifc</ct:filetype>
      </ct:InternalDocument>
    </rdf:RDF>"##
        .replace("__FILENAME__", declared);
    let archive_entry = format!("Payload documents/{entry}");
    zip_with_index(index.as_bytes(), Some(&archive_entry), b"not allowed")
}

#[test]
fn case_folded_and_noncanonical_entry_names_are_rejected() {
    for names in [
        vec!["Payload documents/model.ifc", "Payload documents/MODEL.ifc"],
        vec!["../escape.ifc"],
        vec!["Payload documents\\model.ifc"],
    ] {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("Index.rdf", options).unwrap();
        writer.write_all(MINIMAL_INDEX.as_bytes()).unwrap();
        for name in names {
            writer.start_file(name, options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        assert!(IcddContainer::open_bytes(bytes).is_err());
    }
}

#[test]
fn declared_paths_with_ambiguous_normalization_are_rejected() {
    for path in ["nested//model.ifc", "nested/model.ifc/"] {
        let error = match IcddContainer::open_bytes(icdd_with_declared_and_entry(path, "model.ifc"))
        {
            Ok(_) => panic!("ambiguous declared path must fail closed"),
            Err(error) => error,
        };
        assert!(error
            .to_string()
            .contains("unsafe or noncanonical archive path"));
    }
}

#[test]
fn extraction_rejects_parent_directory_payload_paths() {
    let error = match IcddContainer::open_bytes(icdd_with_filename("../../escape.ifc")) {
        Ok(_) => panic!("unsafe archive path must fail during open"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unsafe or noncanonical"));
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

#[test]
fn duplicate_index_entries_are_rejected() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("Index.rdf", options).unwrap();
    writer.write_all(MINIMAL_INDEX.as_bytes()).unwrap();
    writer.start_file("index.rdf", options).unwrap();
    writer.write_all(MINIMAL_INDEX.as_bytes()).unwrap();
    let bytes = writer.finish().unwrap().into_inner();

    let error = match IcddContainer::open_bytes(bytes) {
        Ok(_) => panic!("duplicate entry must fail"),
        Err(error) => error,
    };
    assert!(error
        .to_string()
        .contains("duplicate case-folded ZIP entry"));
}

#[cfg(unix)]
#[test]
fn extraction_rejects_existing_symlink_ancestors() {
    use std::os::unix::fs::symlink;

    let mut container = IcddContainer::open_bytes(icdd_with_filename("nested/model.ifc")).unwrap();
    let root = std::env::temp_dir().join(format!(
        "openbim-icdd-symlink-safety-{}",
        std::process::id()
    ));
    let extraction = root.join("output");
    let outside = root.join("outside");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&extraction).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    symlink(&outside, extraction.join("nested")).unwrap();

    assert!(container.extract_payloads(&extraction).is_err());
    assert!(!outside.join("model.ifc").exists());

    let _ = std::fs::remove_dir_all(root);
}
