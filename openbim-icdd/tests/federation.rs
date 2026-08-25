use openbim_icdd::{
    parse_poing_federation_icdd, write_poing_federation_icdd, FederationIcddPayload, IcddContainer,
    PoingFederationManifest, PoingFederationMember,
};

fn manifest() -> PoingFederationManifest {
    PoingFederationManifest {
        id: "campus".into(),
        uuid: "019c97c5-a8f4-7000-8000-000000000001".into(),
        name: "Campus".into(),
        primary_member_id: "architecture".into(),
        coordinate_reference_system: Some("https://www.opengis.net/def/crs/EPSG/0/25832".into()),
        members: vec![PoingFederationMember {
            id: "architecture".into(),
            uuid: "019c97c5-a8f4-7000-8000-000000000002".into(),
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

#[test]
fn federation_extension_round_trips_through_canonical_icdd() {
    let manifest = manifest();
    let payloads = [FederationIcddPayload::Internal {
        member_id: "architecture",
        filename: "architecture.ifc",
        bytes: b"ISO-10303-21;\nEND-ISO-10303-21;\n",
    }];
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
fn federation_writer_rejects_non_finite_transforms() {
    let mut manifest = manifest();
    manifest.members[0].transform[3] = f64::NAN;
    assert!(write_poing_federation_icdd(&manifest, &[]).is_err());
}
