use openbim_icdd::{
    parse_poing_federation_icdd, write_poing_federation_icdd, FederationIcddPayload, IcddContainer,
    PoingFederationManifest, PoingFederationMember,
};
use std::io::{Cursor, Read, Write};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn manifest() -> PoingFederationManifest {
    PoingFederationManifest {
        id: "campus".into(),
        uuid: "019c97c5-a8f4-7000-8000-000000000001".parse().unwrap(),
        name: "Campus".into(),
        primary_member_id: "architecture".into(),
        coordinate_reference_system: Some("https://www.opengis.net/def/crs/EPSG/0/25832".into()),
        members: vec![PoingFederationMember {
            id: "architecture".into(),
            uuid: "019c97c5-a8f4-7000-8000-000000000002".parse().unwrap(),
            name: "Architecture".into(),
            source: "architecture.ifc".into(),
            industry_domain: Some("architecture".into()),
            schema: Some("IFC4X3_ADD2".into()),
            content_hash: Some("sha256:abc".into()),
            coordinate_reference_system: None,
            transform: [
                1.0, 0.0, 0.0, 100.0, 0.0, 1.0, 0.0, 200.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }],
    }
}

fn payloads() -> [FederationIcddPayload<'static>; 1] {
    [FederationIcddPayload::Internal {
        member_id: "architecture",
        filename: "architecture.ifc",
        bytes: b"ISO-10303-21;\nEND-ISO-10303-21;\n",
    }]
}

#[test]
fn federation_extension_round_trips_through_canonical_icdd() {
    let manifest = manifest();
    let payloads = payloads();
    let first = write_poing_federation_icdd(&manifest, &payloads).unwrap();
    let second = write_poing_federation_icdd(&manifest, &payloads).unwrap();
    assert_eq!(first, second);

    let mut container = IcddContainer::open_bytes(first.clone()).unwrap();
    assert_eq!(container.ifc_documents().len(), 1);
    let document = container.ifc_documents()[0].clone();
    assert_eq!(
        container.payload_bytes(&document).unwrap(),
        b"ISO-10303-21;\nEND-ISO-10303-21;\n"
    );

    let parsed = parse_poing_federation_icdd(&first).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn federation_writer_rejects_incomplete_bindings() {
    let error =
        write_poing_federation_icdd(&manifest(), &[]).expect_err("missing binding must fail");
    assert!(error.to_string().contains("missing payload binding"));
}

#[test]
fn federation_writer_rejects_source_binding_mismatch() {
    let mut manifest = manifest();
    manifest.members[0].source = "different.ifc".into();
    let error = write_poing_federation_icdd(&manifest, &payloads())
        .expect_err("member source must match its payload binding");
    assert!(error.to_string().contains("does not match bound source"));
}

#[test]
fn federation_writer_rejects_non_finite_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform[3] = f64::NAN;
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_err());
}

#[test]
fn federation_writer_rejects_non_affine_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform[12] = 1.0;
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_err());
}

#[test]
fn federation_writer_rejects_singular_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform[0] = 0.0;
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_err());
}

#[test]
fn federation_writer_rejects_tiny_projective_terms() {
    let mut manifest = manifest();
    manifest.members[0].transform[12] = 1e-13;
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_err());
}

#[test]
fn federation_writer_accepts_small_but_invertible_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform[0] = 1e-13;
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_ok());
}

#[test]
fn federation_writer_rejects_overflowing_singular_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform = [
        1e308, 1e308, 0.0, 0.0, 1e308, 1e308, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    assert!(write_poing_federation_icdd(&manifest, &payloads()).is_err());
}

#[test]
fn federation_parser_rejects_missing_internal_member_payloads() {
    let archive = write_poing_federation_icdd(&manifest(), &payloads()).unwrap();
    let mut input = ZipArchive::new(Cursor::new(archive)).unwrap();
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for index in 0..input.len() {
        let mut entry = input.by_index(index).unwrap();
        if entry.name() == "Payload documents/architecture.ifc" {
            continue;
        }
        if entry.is_dir() {
            output.add_directory(entry.name(), options).unwrap();
        } else {
            output.start_file(entry.name(), options).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            output.write_all(&bytes).unwrap();
        }
    }
    let incomplete = output.finish().unwrap().into_inner();
    let error = parse_poing_federation_icdd(&incomplete)
        .expect_err("missing member payload must fail closed");
    assert!(error.to_string().contains("unavailable internal source"));
}
