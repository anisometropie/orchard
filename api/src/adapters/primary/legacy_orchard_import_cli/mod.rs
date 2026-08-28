use std::{env, path::PathBuf};

pub struct LegacyOrchardImportCommand {
    pub database_url: String,
    pub geojson_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum LegacyOrchardImportCliError {
    DatabaseUrlIsNotConfigured,
}

pub fn command_from_environment() -> Result<LegacyOrchardImportCommand, LegacyOrchardImportCliError>
{
    let database_url = env::var("ORCHARD_DATABASE_URL")
        .map_err(|_| LegacyOrchardImportCliError::DatabaseUrlIsNotConfigured)?;

    Ok(LegacyOrchardImportCommand {
        database_url,
        geojson_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/trees-wgs84.geojson"),
    })
}
