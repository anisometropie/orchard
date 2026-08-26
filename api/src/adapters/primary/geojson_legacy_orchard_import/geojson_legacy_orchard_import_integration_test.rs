use std::path::Path;

use orchard_api::adapters::primary::import_legacy_geojson_file;
use orchard_api::adapters::secondary::InMemoryOrchardStorage;

#[test]
fn import_geojson_file() {
    let (mut orchard_unit_of_work, observed_orchard) = InMemoryOrchardStorage::new();

    let import_result = import_legacy_geojson_file(
        Path::new("../data/trees-wgs84.geojson"),
        &mut orchard_unit_of_work,
    );

    assert_eq!(import_result, Ok(278));
    assert_eq!(observed_orchard.trees().len(), 278);
}
