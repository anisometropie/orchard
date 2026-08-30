use orchard_api::adapters::secondary::InMemoryOrchardStorage;
use orchard_api::hexagon::models::{
    AerialOverlayId, AerialOverlayImage, GeoPoint, MapConfiguration,
};
use orchard_api::hexagon::use_cases::load_aerial_overlay_image::load_aerial_overlay_image;

#[test]
fn load_an_aerial_overlays_image_bytes_and_media_type() {
    let expected = AerialOverlayImage {
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3, 4],
    };
    let mut storage = InMemoryOrchardStorage::with_map_configuration(
        MapConfiguration {
            default_center: GeoPoint {
                longitude: 0.5,
                latitude: 0.5,
            },
            aerial_overlays: vec![],
        },
        vec![(AerialOverlayId(7), expected.clone())],
    );

    let result = load_aerial_overlay_image(AerialOverlayId(7), &mut storage);

    assert_eq!(result, Ok(expected));
}
