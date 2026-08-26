use std::path::Path;

use serde::Deserialize;

use crate::hexagon::ports::OrchardUnitOfWork;
use crate::hexagon::use_cases::import_legacy_orchard::{
    LegacyOrchardImportRequested, LegacyTreeSnapshot, import_legacy_orchard,
};

#[derive(Debug, PartialEq)]
pub enum GeoJsonLegacyOrchardImportError {
    CouldNotReadGeoJson,
    CouldNotParseGeoJson,
    CouldNotImportOrchard,
}

pub fn import_legacy_geojson_file<U>(
    path: &Path,
    orchard_unit_of_work: &mut U,
) -> Result<usize, GeoJsonLegacyOrchardImportError>
where
    U: OrchardUnitOfWork,
{
    let source = std::fs::read_to_string(path)
        .map_err(|_| GeoJsonLegacyOrchardImportError::CouldNotReadGeoJson)?;
    let collection: GeoJsonFeatureCollection = serde_json::from_str(&source)
        .map_err(|_| GeoJsonLegacyOrchardImportError::CouldNotParseGeoJson)?;
    let trees = collection
        .features
        .into_iter()
        .map(|feature| LegacyTreeSnapshot {
            legacy_feature_id: feature.properties.fid,
            longitude: feature.geometry.coordinates[0],
            latitude: feature.geometry.coordinates[1],
            name: feature.properties.name,
            latin_name: feature.properties.latin_name,
            planted_on: feature.properties.planted_on,
            row_name: feature.properties.row_name,
            is_pioneer: feature.properties.is_pioneer,
            is_alive: feature.properties.is_alive,
            harvest_start_day: feature.properties.harvest_start_day,
            harvest_end_day: feature.properties.harvest_end_day,
            adult_height_meters: feature.properties.adult_height_meters,
            adult_width_meters: feature.properties.adult_width_meters,
        })
        .collect();
    import_legacy_orchard(LegacyOrchardImportRequested { trees }, orchard_unit_of_work)
        .map_err(|_| GeoJsonLegacyOrchardImportError::CouldNotImportOrchard)
}

#[derive(Deserialize)]
struct GeoJsonFeatureCollection {
    features: Vec<GeoJsonFeature>,
}

#[derive(Deserialize)]
struct GeoJsonFeature {
    properties: GeoJsonTreeProperties,
    geometry: GeoJsonPoint,
}

#[derive(Deserialize)]
struct GeoJsonTreeProperties {
    fid: u32,
    name: String,
    latin_name: String,
    #[serde(rename = "date")]
    planted_on: Option<String>,
    #[serde(rename = "Line")]
    row_name: String,
    #[serde(rename = "pioneer")]
    is_pioneer: bool,
    #[serde(rename = "alive")]
    is_alive: bool,
    #[serde(rename = "harvest_date_min")]
    harvest_start_day: Option<u16>,
    #[serde(rename = "harvest_date_max")]
    harvest_end_day: Option<u16>,
    #[serde(rename = "adult_height")]
    adult_height_meters: f64,
    #[serde(rename = "adult_width")]
    adult_width_meters: f64,
}

#[derive(Deserialize)]
struct GeoJsonPoint {
    coordinates: [f64; 2],
}
