use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{AerialOverlay, AerialOverlayId, GeoPoint, MapConfiguration};
use orchard_api::hexagon::use_cases::load_map_configuration::load_map_configuration;

#[test]
fn load_the_default_users_map_center_and_aerial_overlays() {
    let expected = MapConfiguration {
        default_center: GeoPoint {
            longitude: 0.5,
            latitude: 0.5,
        },
        aerial_overlays: vec![AerialOverlay {
            id: AerialOverlayId(7),
            name: "Main orchard".into(),
            corners: [
                GeoPoint {
                    longitude: 0.0,
                    latitude: 1.0,
                },
                GeoPoint {
                    longitude: 1.0,
                    latitude: 1.0,
                },
                GeoPoint {
                    longitude: 1.0,
                    latitude: 0.0,
                },
                GeoPoint {
                    longitude: 0.0,
                    latitude: 0.0,
                },
            ],
        }],
    };
    let mut storage = InMemoryOrchardStorage::with_map_configuration(expected.clone(), vec![]);

    let result = load_map_configuration(&mut storage);

    assert_eq!(result, Ok(expected));
}
