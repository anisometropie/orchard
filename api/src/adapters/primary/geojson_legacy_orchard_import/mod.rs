use std::path::Path;

use serde::Deserialize;

use crate::hexagon::models::{
    BotanicalTaxon, IdentificationStatus, InfraspecificRank, InfraspecificTaxon,
    LegacyPlantIdentification, LegacyTreeSource, NamedTaxon, PlantIdentity, ReproductiveRole,
};
use crate::hexagon::ports::OrchardUnitOfWork;
use crate::hexagon::use_cases::import_legacy_orchard::{
    LegacyOrchardImportError, LegacyOrchardImportRequested, LegacyTreeSnapshot,
    import_legacy_orchard,
};

#[derive(Debug, PartialEq)]
pub enum GeoJsonLegacyOrchardImportError {
    CouldNotReadGeoJson,
    CouldNotParseGeoJson,
    CouldNotParsePlantIdentity { legacy_feature_id: u32 },
    CouldNotImportOrchard(LegacyOrchardImportError),
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
    let tree_collection: GeoJsonTreeFeatureCollection = serde_json::from_str(&source)
        .map_err(|_| GeoJsonLegacyOrchardImportError::CouldNotParseGeoJson)?;
    let trees = tree_collection
        .features
        .into_iter()
        .map(|feature| {
            let properties = feature.properties;
            let legacy_feature_id = properties.fid;
            let plant_identity = parse_legacy_tree_identity(
                &properties.name,
                &properties.latin_name,
                properties.trade_name.clone(),
            )
            .map_err(|_| {
                GeoJsonLegacyOrchardImportError::CouldNotParsePlantIdentity { legacy_feature_id }
            })?;
            Ok(LegacyTreeSnapshot {
                legacy_source: LegacyTreeSource {
                    feature_id: legacy_feature_id,
                    name: properties.name,
                    latin_name: properties.latin_name,
                    legacy_identification: properties.legacy_identification.map(
                        |legacy_identification| LegacyPlantIdentification {
                            name: legacy_identification.name,
                            latin_name: legacy_identification.latin_name,
                        },
                    ),
                    source_url: properties.source_url,
                },
                longitude: feature.geometry.coordinates[0],
                latitude: feature.geometry.coordinates[1],
                plant_identity,
                planted_on: properties.planted_on,
                row_name: properties.row_name,
                is_pioneer: properties.is_pioneer,
                is_alive: properties.is_alive,
                reproductive_role: properties.reproductive_role,
                harvest_start_day: properties.harvest_start_day,
                harvest_end_day: properties.harvest_end_day,
                adult_height_meters: properties.adult_height_meters,
                adult_width_meters: properties.adult_width_meters,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    import_legacy_orchard(LegacyOrchardImportRequested { trees }, orchard_unit_of_work)
        .map_err(GeoJsonLegacyOrchardImportError::CouldNotImportOrchard)
}

#[derive(Deserialize)]
struct GeoJsonTreeFeatureCollection {
    features: Vec<GeoJsonTreeFeature>,
}

#[derive(Deserialize)]
struct GeoJsonTreeFeature {
    properties: GeoJsonTreeProperties,
    geometry: GeoJsonPoint,
}

#[derive(Deserialize)]
struct GeoJsonTreeProperties {
    fid: u32,
    name: String,
    latin_name: String,
    trade_name: Option<String>,
    legacy_identification: Option<GeoJsonLegacyPlantIdentification>,
    #[serde(rename = "source")]
    source_url: Option<String>,
    #[serde(rename = "date")]
    planted_on: Option<String>,
    #[serde(rename = "Line")]
    row_name: String,
    #[serde(rename = "pioneer")]
    is_pioneer: bool,
    #[serde(rename = "alive")]
    is_alive: bool,
    reproductive_role: Option<ReproductiveRole>,
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
struct GeoJsonLegacyPlantIdentification {
    name: String,
    latin_name: String,
}

#[derive(Deserialize)]
struct GeoJsonPoint {
    coordinates: [f64; 2],
}

#[derive(Debug, PartialEq)]
enum LegacyPlantIdentityParseError {
    UnsupportedSyntax,
}

fn parse_legacy_tree_identity(
    legacy_name: &str,
    legacy_botanical_name: &str,
    trade_name: Option<String>,
) -> Result<PlantIdentity, LegacyPlantIdentityParseError> {
    let (taxon, cultivar) = split_cultivar(legacy_botanical_name)?;
    let common_name = cultivar.as_deref().map_or(legacy_name, |cultivar| {
        remove_quoted_label_suffix(legacy_name, cultivar)
    });
    let common_name = trade_name
        .as_deref()
        .map_or(common_name, |trade_name| {
            remove_trade_name_suffix(common_name, trade_name)
        })
        .into();
    let (botanical_taxon, identification_status) = parse_legacy_botanical_taxon(taxon)?;

    Ok(PlantIdentity {
        common_name,
        botanical_taxon,
        cultivar,
        trade_name,
        identification_status,
    })
}

fn remove_quoted_label_suffix<'a>(name: &'a str, label: &str) -> &'a str {
    let suffix = format!(" ‘{label}’");
    name.strip_suffix(&suffix)
        .filter(|common_name| !common_name.is_empty())
        .unwrap_or(name)
}

fn remove_trade_name_suffix<'a>(name: &'a str, trade_name: &str) -> &'a str {
    let unquoted_suffix = format!(" {trade_name}");
    name.strip_suffix(&unquoted_suffix)
        .or_else(|| {
            let without_quoted_trade_name = remove_quoted_label_suffix(name, trade_name);
            (without_quoted_trade_name != name).then_some(without_quoted_trade_name)
        })
        .filter(|common_name| !common_name.is_empty())
        .unwrap_or(name)
}

fn parse_legacy_botanical_taxon(
    taxon: &str,
) -> Result<(BotanicalTaxon, IdentificationStatus), LegacyPlantIdentityParseError> {
    let (taxon, identification_status) = match taxon.strip_suffix(" ?") {
        Some(taxon) => (taxon, IdentificationStatus::Uncertain),
        None => (taxon, IdentificationStatus::Confirmed),
    };
    let botanical_taxon = match taxon.split_whitespace().collect::<Vec<_>>().as_slice() {
        [genus, group_name, "Group)"] if group_name.starts_with('(') => {
            let group_name = group_name
                .strip_prefix('(')
                .filter(|name| !name.is_empty())
                .ok_or(LegacyPlantIdentityParseError::UnsupportedSyntax)?;
            BotanicalTaxon::Named(NamedTaxon {
                genus: (*genus).into(),
                species: None,
                species_is_hybrid: false,
                infraspecific: None,
                is_aggregate: false,
                cultivar_group: Some(group_name.into()),
            })
        }
        [genus, species, "var.", infraspecific_name] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: Some((*species).into()),
            species_is_hybrid: false,
            infraspecific: Some(InfraspecificTaxon {
                rank: InfraspecificRank::Variety,
                name: (*infraspecific_name).into(),
            }),
            is_aggregate: false,
            cultivar_group: None,
        }),
        [genus, species, "subsp.", infraspecific_name] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: Some((*species).into()),
            species_is_hybrid: false,
            infraspecific: Some(InfraspecificTaxon {
                rank: InfraspecificRank::Subspecies,
                name: (*infraspecific_name).into(),
            }),
            is_aggregate: false,
            cultivar_group: None,
        }),
        [genus, species, "agg."] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: Some((*species).into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: true,
            cultivar_group: None,
        }),
        [genus, "×", hybrid_species] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: Some((*hybrid_species).into()),
            species_is_hybrid: true,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        [
            first_genus,
            first_species,
            "×",
            second_genus,
            second_species,
        ] => BotanicalTaxon::HybridFormula {
            parents: [
                NamedTaxon {
                    genus: (*first_genus).into(),
                    species: Some((*first_species).into()),
                    species_is_hybrid: false,
                    infraspecific: None,
                    is_aggregate: false,
                    cultivar_group: None,
                },
                NamedTaxon {
                    genus: (*second_genus).into(),
                    species: Some((*second_species).into()),
                    species_is_hybrid: false,
                    infraspecific: None,
                    is_aggregate: false,
                    cultivar_group: None,
                },
            ],
        },
        [genus] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: None,
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        [genus, species] => BotanicalTaxon::Named(NamedTaxon {
            genus: (*genus).into(),
            species: Some((*species).into()),
            species_is_hybrid: false,
            infraspecific: None,
            is_aggregate: false,
            cultivar_group: None,
        }),
        _ => return Err(LegacyPlantIdentityParseError::UnsupportedSyntax),
    };

    Ok((botanical_taxon, identification_status))
}

fn split_cultivar(
    legacy_botanical_name: &str,
) -> Result<(&str, Option<String>), LegacyPlantIdentityParseError> {
    let Some(without_closing_quote) = legacy_botanical_name.strip_suffix('’') else {
        return Ok((legacy_botanical_name, None));
    };
    let Some(opening_quote_position) = without_closing_quote.rfind('‘') else {
        return Err(LegacyPlantIdentityParseError::UnsupportedSyntax);
    };
    let cultivar = &without_closing_quote[opening_quote_position + '‘'.len_utf8()..];
    let taxon = without_closing_quote[..opening_quote_position].trim_end();
    if cultivar.is_empty() || taxon.is_empty() {
        return Err(LegacyPlantIdentityParseError::UnsupportedSyntax);
    }
    Ok((taxon, Some(cultivar.into())))
}

#[cfg(test)]
mod legacy_plant_identity_unit_test;
