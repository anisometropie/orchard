use std::process::ExitCode;

use orchard_api::adapters::primary::geojson_legacy_orchard_import::GeoJsonLegacyOrchardImportError;
use orchard_api::adapters::primary::legacy_orchard_import_cli::{
    LegacyOrchardImportCliError, command_from_environment,
};
use orchard_api::bootstrap::{LegacyOrchardImportCommandError, import_legacy_orchard};
use orchard_api::hexagon::use_cases::import_legacy_orchard::LegacyOrchardImportError;

fn main() -> ExitCode {
    let command = match command_from_environment() {
        Ok(command) => command,
        Err(LegacyOrchardImportCliError::DatabaseUrlIsNotConfigured) => {
            eprintln!("ORCHARD_DATABASE_URL is not configured.");
            return ExitCode::from(2);
        }
    };

    match import_legacy_orchard(command) {
        Ok(imported_tree_count) => {
            println!("Imported {imported_tree_count} trees.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Import failed: {}", import_error_message(error));
            ExitCode::from(1)
        }
    }
}

fn import_error_message(error: LegacyOrchardImportCommandError) -> String {
    match error {
        LegacyOrchardImportCommandError::CouldNotConnectToDatabase => {
            "could not connect to the orchard database.".into()
        }
        LegacyOrchardImportCommandError::CouldNotImportOrchard(
            GeoJsonLegacyOrchardImportError::CouldNotImportOrchard(
                LegacyOrchardImportError::LegacyFeatureAlreadyImported { legacy_feature_id },
            ),
        ) => {
            format!("legacy feature {legacy_feature_id} is already imported. No changes were made.")
        }
        LegacyOrchardImportCommandError::CouldNotImportOrchard(
            GeoJsonLegacyOrchardImportError::CouldNotReadGeoJson,
        ) => "could not read the legacy GeoJSON file.".into(),
        LegacyOrchardImportCommandError::CouldNotImportOrchard(
            GeoJsonLegacyOrchardImportError::CouldNotParseGeoJson,
        ) => "could not parse the legacy GeoJSON file.".into(),
        LegacyOrchardImportCommandError::CouldNotImportOrchard(
            GeoJsonLegacyOrchardImportError::CouldNotParsePlantIdentity { legacy_feature_id },
        ) => format!("could not parse plant identity for legacy feature {legacy_feature_id}."),
        LegacyOrchardImportCommandError::CouldNotImportOrchard(
            GeoJsonLegacyOrchardImportError::CouldNotImportOrchard(_),
        ) => "the orchard could not be imported. No changes were made.".into(),
    }
}
