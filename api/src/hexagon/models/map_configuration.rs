#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AerialOverlayId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AerialOverlay {
    pub id: AerialOverlayId,
    pub name: String,
    pub corners: [GeoPoint; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapConfiguration {
    pub default_center: GeoPoint,
    pub aerial_overlays: Vec<AerialOverlay>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AerialOverlayImage {
    pub media_type: String,
    pub bytes: Vec<u8>,
}
