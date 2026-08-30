use std::path::Path;

use orchard_api::adapters::primary::import_legacy_geojson_file;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;

#[test]
fn import_synthetic_geojson_file() {
    let (mut orchard_storage, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_geojson_file(
        Path::new("src/adapters/primary/orchard_cli/one-tree.geojson"),
        &mut orchard_storage,
    );

    assert_eq!(import_result, Ok(1));
    assert_eq!(observed_orchard.plant_identities().len(), 1);
    let trees = observed_orchard.trees();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0].longitude, 0.72);
    assert_eq!(trees[0].latitude, 0.24);
}
