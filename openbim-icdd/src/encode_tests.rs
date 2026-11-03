//! Round-trip, determinism, error and container-level tests for the typed encoders.

use crate::index::parse_index;
use crate::linkset::parse_linkset;
use crate::schema::*;
use crate::{IcddArchiveBuilder, IcddContainer, IcddError};

const BASE: &str = "https://example.org/container/";

fn id(local: &str) -> String {
    format!("{BASE}{local}")
}

fn doc(local: &str, kind: DocumentKind) -> Document {
    Document {
        id: id(local),
        kind,
        name: None,
        description: None,
        filetype: None,
        format: None,
        checksum: None,
        encrypted: false,
        requested: false,
    }
}

fn internal(local: &str, filename: &str) -> Document {
    doc(
        local,
        DocumentKind::Internal {
            filename: filename.into(),
        },
    )
}

/// Documents are listed in id order, which is the order the reader returns.
fn every_document_kind() -> Vec<Document> {
    vec![
        Document {
            name: Some("Architecture".into()),
            description: Some("Design model".into()),
            filetype: Some("ifc".into()),
            format: Some("application/x-extension-ifc".into()),
            ..internal("a-model", "model.ifc")
        },
        Document {
            checksum: Some(Checksum {
                algorithm: "SHA-256".into(),
                value: "abc123".into(),
            }),
            ..internal("b-secured", "secured.pdf")
        },
        Document {
            encrypted: true,
            ..internal("c-encrypted", "encrypted.bin")
        },
        Document {
            checksum: Some(Checksum {
                algorithm: "MD5".into(),
                value: "ff".into(),
            }),
            encrypted: true,
            ..internal("d-both", "both.bin")
        },
        Document {
            requested: true,
            ..internal("e-requested", "later.xlsx")
        },
        doc(
            "f-external",
            DocumentKind::External {
                url: "https://example.org/spec.pdf".into(),
            },
        ),
        Document {
            requested: true,
            ..doc(
                "g-folder",
                DocumentKind::Folder {
                    foldername: "shapes".into(),
                },
            )
        },
    ]
}

fn index_with(documents: Vec<Document>, linksets: &[&str]) -> ContainerIndex {
    ContainerIndex {
        description: ContainerDescription {
            id: id("container"),
            conformance_indicator: Some("ICDD-Part1-Container".into()),
            description: Some("Typed container".into()),
            creation_date: Some("2026-11-03T10:00:00".into()),
        },
        documents,
        linkset_files: linksets
            .iter()
            .enumerate()
            .map(|(n, filename)| LinksetRef {
                id: id(&format!("linkset-{n}")),
                filename: Some((*filename).into()),
                name: Some(format!("Linkset {n}")),
            })
            .collect(),
    }
}

fn element(local: &str, document: &str, identifier: Option<ElementIdentifier>) -> LinkElement {
    LinkElement {
        id: id(local),
        document_id: Some(id(document)),
        identifier,
    }
}

/// Links in the reader's order: undirected, `DirectedLink`, `Directed1toNLink`.
fn every_link_kind() -> LinkSet {
    let string = ElementIdentifier::String {
        value: "2O2Fr$t4X7Zf8NOew3FLOH".into(),
        field: Some("GlobalId".into()),
    };
    let string_no_field = ElementIdentifier::String {
        value: "row-7".into(),
        field: None,
    };
    let uri = ElementIdentifier::Uri {
        uri: "https://example.org/spec.pdf#page=3".into(),
    };
    let query = ElementIdentifier::Query {
        language: Some("SPARQL".into()),
        expression: Some("SELECT ?s WHERE { ?s ?p ?o }".into()),
    };
    let xpath = ElementIdentifier::Query {
        language: Some("XPath".into()),
        expression: Some("//wall".into()),
    };
    LinkSet {
        filename: "links.rdf".into(),
        links: vec![
            Link {
                id: id("link-1-undirected"),
                directed: false,
                elements: vec![
                    element("l1-e1", "a-model", Some(string)),
                    element("l1-e2", "f-external", Some(uri)),
                    element("l1-e3", "e-requested", None),
                ],
                from: vec![],
                to: vec![],
            },
            Link {
                id: id("link-2-directed"),
                directed: true,
                elements: vec![
                    element("l2-e1", "a-model", Some(string_no_field.clone())),
                    element("l2-e2", "b-secured", Some(query)),
                    element("l2-e3", "g-folder", Some(xpath)),
                ],
                from: vec![id("l2-e1"), id("l2-e2")],
                to: vec![id("l2-e3")],
            },
            Link {
                id: id("link-3-many-to-many"),
                directed: true,
                elements: vec![
                    element("l3-e1", "c-encrypted", None),
                    element("l3-e2", "d-both", None),
                    element("l3-e3", "c-encrypted", Some(string_no_field.clone())),
                    element("l3-e4", "d-both", Some(string_no_field.clone())),
                ],
                from: vec![id("l3-e1"), id("l3-e2")],
                to: vec![id("l3-e3"), id("l3-e4")],
            },
            Link {
                id: id("link-4-one-to-n"),
                directed: true,
                elements: vec![
                    element("l4-e1", "a-model", None),
                    element("l4-e2", "d-both", None),
                    element("l4-e3", "e-requested", None),
                ],
                from: vec![id("l4-e1")],
                to: vec![id("l4-e2"), id("l4-e3")],
            },
        ],
    }
}

#[test]
fn index_round_trips_every_document_kind() {
    let index = index_with(every_document_kind(), &["links.rdf", "more.rdf"]);
    let bytes = index.to_rdf_xml(BASE).unwrap();
    assert_eq!(parse_index(&bytes).unwrap(), index);
}

#[test]
fn bare_index_round_trips() {
    let mut index = index_with(vec![], &[]);
    index.description.description = None;
    index.description.creation_date = None;
    index.description.conformance_indicator = None;
    let bytes = index.to_rdf_xml(BASE).unwrap();
    assert_eq!(parse_index(&bytes).unwrap(), index);
}

#[test]
fn linkset_round_trips_every_link_and_identifier_kind() {
    let index = index_with(every_document_kind(), &["links.rdf"]);
    let linkset = every_link_kind();
    let bytes = linkset.to_rdf_xml(&index, BASE).unwrap();
    assert_eq!(parse_linkset("links.rdf", &bytes).unwrap(), linkset);
}

#[test]
fn relative_ids_resolve_against_the_base() {
    let mut index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    let mut expected = index.clone();
    index.description.id = "#container".into();
    index.documents[0].id = "a-model".into();
    index.linkset_files[0].id = "#linkset-0".into();
    expected.description.id = format!("{BASE}#container");
    expected.documents[0].id = id("a-model");
    expected.linkset_files[0].id = format!("{BASE}#linkset-0");
    let bytes = index.to_rdf_xml(BASE).unwrap();
    assert_eq!(parse_index(&bytes).unwrap(), expected);

    let linkset = LinkSet {
        filename: "links.rdf".into(),
        links: vec![Link {
            id: "#link".into(),
            directed: false,
            elements: vec![
                LinkElement {
                    id: "#element".into(),
                    document_id: Some("a-model".into()),
                    identifier: None,
                },
                LinkElement {
                    id: "#other".into(),
                    document_id: Some("a-model".into()),
                    identifier: Some(ElementIdentifier::String {
                        value: "x".into(),
                        field: None,
                    }),
                },
            ],
            from: vec![],
            to: vec![],
        }],
    };
    let parsed = parse_linkset("links.rdf", &linkset.to_rdf_xml(&index, BASE).unwrap()).unwrap();
    assert_eq!(parsed.links[0].id, format!("{BASE}#link"));
    assert_eq!(parsed.links[0].elements[0].id, format!("{BASE}#element"));
    assert_eq!(parsed.links[0].elements[0].document_id, Some(id("a-model")));
}

#[test]
fn blank_node_ids_round_trip() {
    let index = index_with(vec![internal("a-model", "model.ifc")], &[]);
    let linkset = LinkSet {
        filename: "links.rdf".into(),
        links: vec![Link {
            id: "_:link".into(),
            directed: true,
            elements: vec![
                LinkElement {
                    id: "_:from".into(),
                    document_id: Some(id("a-model")),
                    identifier: Some(ElementIdentifier::String {
                        value: "x".into(),
                        field: None,
                    }),
                },
                LinkElement {
                    id: "_:to".into(),
                    document_id: Some(id("a-model")),
                    identifier: None,
                },
            ],
            from: vec!["_:from".into()],
            to: vec!["_:to".into()],
        }],
    };
    let bytes = linkset.to_rdf_xml(&index, BASE).unwrap();
    assert_eq!(parse_linkset("links.rdf", &bytes).unwrap(), linkset);
}

#[test]
fn output_does_not_depend_on_input_order() {
    let index = index_with(every_document_kind(), &["links.rdf", "more.rdf"]);
    let mut shuffled = index.clone();
    shuffled.documents.reverse();
    shuffled.linkset_files.reverse();
    assert_eq!(
        index.to_rdf_xml(BASE).unwrap(),
        shuffled.to_rdf_xml(BASE).unwrap()
    );

    let linkset = every_link_kind();
    let mut shuffled = linkset.clone();
    shuffled.links.reverse();
    for link in &mut shuffled.links {
        link.elements.reverse();
    }
    assert_eq!(
        linkset.to_rdf_xml(&index, BASE).unwrap(),
        shuffled.to_rdf_xml(&index, BASE).unwrap()
    );
}

#[test]
fn link_types_follow_the_directedness_rules() {
    let index = index_with(every_document_kind(), &["links.rdf"]);
    let text = String::from_utf8(every_link_kind().to_rdf_xml(&index, BASE).unwrap()).unwrap();
    assert_eq!(text.matches("<ls:Link ").count(), 1);
    assert_eq!(text.matches("<ls:DirectedLink ").count(), 2);
    assert_eq!(text.matches("<ls:Directed1toNLink ").count(), 1);
    // Directed links assert hasLinkElement as well as from/to: 3+3+4+3 elements.
    assert_eq!(text.matches("<ls:hasLinkElement ").count(), 13);
}

fn expect_invalid<T: std::fmt::Debug>(result: Result<T, IcddError>, needle: &str) {
    match result {
        Err(IcddError::NotConformant(message)) => {
            assert!(message.contains(needle), "{message:?} lacks {needle:?}")
        }
        other => panic!("expected NotConformant containing {needle:?}, got {other:?}"),
    }
}

fn one_link(link: Link) -> LinkSet {
    LinkSet {
        filename: "links.rdf".into(),
        links: vec![link],
    }
}

fn simple_link() -> Link {
    Link {
        id: id("link"),
        directed: true,
        elements: vec![
            element("e1", "a-model", None),
            element(
                "e2",
                "a-model",
                Some(ElementIdentifier::String {
                    value: "x".into(),
                    field: None,
                }),
            ),
        ],
        from: vec![id("e1")],
        to: vec![id("e2")],
    }
}

#[test]
fn rejects_invalid_linksets() {
    let index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    let encode = |link: Link| one_link(link).to_rdf_xml(&index, BASE);

    let mut link = simple_link();
    link.elements[1].document_id = Some(id("missing"));
    expect_invalid(encode(link), "not a document of the index");

    let mut link = simple_link();
    link.elements[0].document_id = None;
    expect_invalid(encode(link), "has no document");

    let mut link = simple_link();
    link.elements.truncate(1);
    link.from = vec![id("e1")];
    link.to.clear();
    expect_invalid(encode(link), "at least two link elements");

    let mut link = simple_link();
    link.elements[1] = LinkElement {
        id: id("e2"),
        ..link.elements[0].clone()
    };
    expect_invalid(encode(link), "duplicate link elements");

    let mut link = simple_link();
    link.from.clear();
    expect_invalid(encode(link), "at least one from and one to");

    let mut link = simple_link();
    link.to.clear();
    expect_invalid(encode(link), "at least one from and one to");

    let mut link = simple_link();
    link.to = vec![id("e2"), id("e2")];
    expect_invalid(encode(link), "twice");

    let mut link = simple_link();
    link.elements[0].identifier = Some(ElementIdentifier::Query {
        language: Some("XPath".into()),
        expression: None,
    });
    expect_invalid(encode(link), "queryLanguage and ls:queryExpression");

    let mut link = simple_link();
    link.elements[0].identifier = Some(ElementIdentifier::Query {
        language: None,
        expression: Some("//x".into()),
    });
    expect_invalid(encode(link), "queryLanguage and ls:queryExpression");

    let mut link = simple_link();
    link.elements[0].identifier = Some(ElementIdentifier::String {
        value: "a\u{1}b".into(),
        field: None,
    });
    expect_invalid(encode(link), "control character");

    for label in ["_:1", "_:a b", "_:", "_:a:b"] {
        let mut link = simple_link();
        link.id = label.into();
        expect_invalid(encode(link), "NCName");
    }

    let mut link = simple_link();
    link.directed = false;
    expect_invalid(encode(link), "undirected link");

    let mut link = simple_link();
    link.to = vec![id("elsewhere")];
    expect_invalid(encode(link), "not one of its elements");

    let mut link = simple_link();
    link.elements[1].id = link.elements[0].id.clone();
    expect_invalid(encode(link), "duplicate node id");

    let mut link = simple_link();
    link.id = id("e1");
    expect_invalid(encode(link), "duplicate node id");

    expect_invalid(
        one_link(simple_link()).to_rdf_xml(&index, "not an iri"),
        "invalid base IRI",
    );
}

#[test]
fn rejects_invalid_indexes() {
    let valid = || index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    expect_invalid(valid().to_rdf_xml("not an iri"), "invalid base IRI");

    let mut index = valid();
    index.documents.push(index.documents[0].clone());
    expect_invalid(index.to_rdf_xml(BASE), "duplicate node id");

    let mut index = valid();
    index.documents[0].kind = DocumentKind::Internal {
        filename: String::new(),
    };
    expect_invalid(index.to_rdf_xml(BASE), "empty filename");

    let mut index = valid();
    index.documents[0].checksum = Some(Checksum {
        algorithm: String::new(),
        value: "x".into(),
    });
    expect_invalid(index.to_rdf_xml(BASE), "empty checksum");

    for bad in [
        "yesterday",
        "2026-13-01T00:00:00",
        "2026-02-30T00:00:00",
        "2026-11-03",
        "2026-11-03T25:00:00",
        "2026-11-03T10:00:00+15:00",
    ] {
        let mut index = valid();
        index.description.creation_date = Some(bad.into());
        expect_invalid(index.to_rdf_xml(BASE), "xsd:dateTime");
    }
    for good in [
        "2026-11-03T10:00:00Z",
        "2024-02-29T23:59:59.5+01:00",
        "2026-11-03T24:00:00",
    ] {
        let mut index = valid();
        index.description.creation_date = Some(good.into());
        let bytes = index.to_rdf_xml(BASE).unwrap();
        assert_eq!(parse_index(&bytes).unwrap(), index);
    }

    let mut index = valid();
    index.documents[0].name = Some("bell\u{7}".into());
    expect_invalid(index.to_rdf_xml(BASE), "control character");

    let mut index = valid();
    index.linkset_files[0].filename = None;
    expect_invalid(index.to_rdf_xml(BASE), "no ct:filename");

    let mut index = valid();
    index.linkset_files.push(index.linkset_files[0].clone());
    index.linkset_files[1].id = id("linkset-again");
    expect_invalid(index.to_rdf_xml(BASE), "duplicate linkset filename");
}

#[test]
fn builder_rejects_a_missing_indicator_and_an_undeclared_linkset() {
    let mut index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    expect_invalid(
        IcddArchiveBuilder::new(index.to_rdf_xml(BASE).unwrap())
            .unwrap()
            .add_typed_linkset(&one_link(simple_link())),
        "needs a builder made with from_index",
    );
    index.description.conformance_indicator = None;
    expect_invalid(
        IcddArchiveBuilder::from_index(&index, BASE),
        "conformanceIndicator",
    );

    let index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    let mut linkset = one_link(simple_link());
    linkset.filename = "other.rdf".into();
    let builder = IcddArchiveBuilder::from_index(&index, BASE).unwrap();
    expect_invalid(
        builder.add_typed_linkset(&linkset),
        "declares no linkset file",
    );
}

#[test]
fn typed_container_is_conformant_when_written_and_read_back() {
    let index = index_with(every_document_kind(), &["links.rdf"]);
    let linkset = every_link_kind();
    let bytes = IcddArchiveBuilder::from_index(&index, BASE)
        .unwrap()
        .add_payload("model.ifc", b"ISO-10303-21;\n")
        .unwrap()
        .add_payload("secured.pdf", b"%PDF")
        .unwrap()
        .add_payload("encrypted.bin", b"\0")
        .unwrap()
        .add_payload("both.bin", b"\0\0")
        .unwrap()
        .add_typed_linkset(&linkset)
        .unwrap()
        .finish()
        .unwrap();

    let container = IcddContainer::open_bytes(bytes.clone()).unwrap();
    assert_eq!(container.conformance_issues(), Vec::<String>::new());
    assert_eq!(container.index(), &index);
    assert_eq!(container.linksets(), [linkset]);

    let again = IcddArchiveBuilder::from_index(&index, BASE)
        .unwrap()
        .add_payload("model.ifc", b"ISO-10303-21;\n")
        .unwrap()
        .add_payload("secured.pdf", b"%PDF")
        .unwrap()
        .add_payload("encrypted.bin", b"\0")
        .unwrap()
        .add_payload("both.bin", b"\0\0")
        .unwrap()
        .add_typed_linkset(&every_link_kind())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(bytes, again);
}

#[test]
fn conformance_issues_report_links_to_documents_outside_the_index() {
    let index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    let mut link = simple_link();
    link.elements[1].document_id = Some(id("elsewhere"));
    // The typed API refuses this, so go through the raw linkset path.
    let rdf = {
        let mut wide = index.clone();
        wide.documents.push(internal("elsewhere", "other.ifc"));
        one_link(link).to_rdf_xml(&wide, BASE).unwrap()
    };
    let bytes = IcddArchiveBuilder::from_index(&index, BASE)
        .unwrap()
        .add_payload("model.ifc", b"x")
        .unwrap()
        .add_linkset("links.rdf", rdf)
        .unwrap()
        .finish()
        .unwrap();
    let container = IcddContainer::open_bytes(bytes).unwrap();
    let issues = container.conformance_issues();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert!(
        issues[0].contains("not a document of the index"),
        "{issues:?}"
    );
}

#[test]
fn builder_checks_linksets_against_the_stored_index() {
    let index = index_with(vec![internal("a-model", "model.ifc")], &["links.rdf"]);
    let mut link = simple_link();
    link.elements[1].document_id = Some(id("elsewhere"));
    expect_invalid(
        IcddArchiveBuilder::from_index(&index, BASE)
            .unwrap()
            .add_typed_linkset(&one_link(link)),
        "not a document of the index",
    );
}

/// Small deterministic generator (LCG): every combination of document kind,
/// mix-ins, link kind and identifier kind, with values in the reader's order.
#[test]
fn generated_values_round_trip() {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move |bound: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % bound
    };

    let mut documents = Vec::new();
    for kind in 0..3 {
        for secured in [false, true] {
            for encrypted in [false, true] {
                for requested in [false, true] {
                    let n = documents.len();
                    let local = format!("doc-{n:02}");
                    let kind = match kind {
                        0 => DocumentKind::Internal {
                            filename: format!("f{n}.bin"),
                        },
                        1 => DocumentKind::External {
                            url: format!("https://example.org/{n}"),
                        },
                        _ => DocumentKind::Folder {
                            foldername: format!("dir{n}"),
                        },
                    };
                    documents.push(Document {
                        name: (next(2) == 0).then(|| format!("name {n}")),
                        description: (next(2) == 0).then(|| format!("description {n}")),
                        filetype: (next(2) == 0).then(|| "ifc".into()),
                        format: (next(2) == 0).then(|| "application/x".into()),
                        checksum: secured.then(|| Checksum {
                            algorithm: "SHA-256".into(),
                            value: format!("{:x}", next(u64::MAX)),
                        }),
                        encrypted,
                        requested,
                        ..doc(&local, kind)
                    });
                }
            }
        }
    }
    let count = documents.len() as u64;
    let index = index_with(documents, &["gen.rdf"]);
    let parsed = parse_index(&index.to_rdf_xml(BASE).unwrap()).unwrap();
    assert_eq!(parsed, index);

    let identifier = |kind: u64, n: u64| match kind {
        0 => None,
        1 => Some(ElementIdentifier::String {
            value: format!("guid-{n}"),
            field: None,
        }),
        2 => Some(ElementIdentifier::String {
            value: format!("guid-{n}"),
            field: Some("GlobalId".into()),
        }),
        3 => Some(ElementIdentifier::Uri {
            uri: format!("https://example.org/doc#{n}"),
        }),
        _ => Some(ElementIdentifier::Query {
            language: Some("SPARQL".into()),
            expression: Some(format!("SELECT {n}")),
        }),
    };

    // Reader order: undirected, DirectedLink, Directed1toNLink.
    let mut links = Vec::new();
    for group in 0..3_u64 {
        for n in 0..24_u64 {
            let elements_len = 2 + next(3);
            let first = next(count);
            let elements: Vec<LinkElement> = (0..elements_len)
                .map(|k| {
                    let document = (first + k * 7) % count;
                    LinkElement {
                        id: id(&format!("g{group}-l{n:02}-e{k}")),
                        document_id: Some(id(&format!("doc-{document:02}"))),
                        identifier: identifier(next(5), k),
                    }
                })
                .collect();
            let ids: Vec<String> = elements.iter().map(|e| e.id.clone()).collect();
            let (directed, from, to) = match group {
                0 => (false, vec![], vec![]),
                // from has two elements: DirectedLink.
                1 => (true, ids[..2].to_vec(), ids[1..].to_vec()),
                // from has one element: Directed1toNLink.
                _ => (true, ids[..1].to_vec(), ids[1..].to_vec()),
            };
            links.push(Link {
                id: id(&format!("link-{group}-{n:02}")),
                directed,
                elements,
                from,
                to,
            });
        }
    }
    let linkset = LinkSet {
        filename: "gen.rdf".into(),
        links,
    };
    let bytes = linkset.to_rdf_xml(&index, BASE).unwrap();
    assert_eq!(parse_linkset("gen.rdf", &bytes).unwrap(), linkset);
}
