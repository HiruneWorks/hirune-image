#[test]
fn committed_fixture_hashes_match_images_goldens_and_provenance() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for line in include_str!("fixtures/SHA256SUMS").lines() {
        let (expected, name) = line.split_once("  ").unwrap();
        let bytes = std::fs::read(root.join(name)).unwrap();
        assert_eq!(
            application::images::sha256(&bytes),
            expected,
            "fixture changed: {name}"
        );
    }
}
