use std::path::Path;

use orchard_api::adapters::primary::import_legacy_geojson_file;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;

#[test]
fn when_the_existing_orchard_geojson_is_imported_every_legacy_tree_is_committed() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_geojson_file(
        Path::new("../data/trees-wgs84.geojson"),
        &mut orchard_unit_of_work,
    );

    assert_eq!(import_result, Ok(278));
    assert_eq!(observed_orchard.trees().len(), 278);
}
