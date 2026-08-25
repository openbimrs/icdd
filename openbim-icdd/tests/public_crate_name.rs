use icdd::INDEX_PATH;

#[test]
fn package_exposes_the_icdd_crate_name() {
    assert_eq!(INDEX_PATH, "Index.rdf");
}
