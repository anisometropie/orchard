use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

use orchard_api::adapters::primary::geojson_legacy_orchard_import::GeoJsonLegacyOrchardImportError;
use orchard_api::adapters::primary::http::start_http_server;
use orchard_api::adapters::primary::import_legacy_geojson_file;
use orchard_api::adapters::primary::orchard_cli::{OrchardCommand, parse_command};
use orchard_api::adapters::secondary::{PostgresMigrator, PostgresOrchardStorage};
use orchard_api::hexagon::use_cases::import_legacy_orchard::LegacyOrchardImportError;

fn main() -> ExitCode {
    let command = match parse_command(env::args_os()) {
        Ok(command) => command,
        Err(error) => {
            let exit_code = error.exit_code();
            let _ = error.print();
            return ExitCode::from(exit_code as u8);
        }
    };
    let database_url = match env::var("ORCHARD_DATABASE_URL") {
        Ok(database_url) => database_url,
        Err(_) => {
            eprintln!("ORCHARD_DATABASE_URL is not configured.");
            return ExitCode::from(2);
        }
    };

    match command {
        OrchardCommand::Migrate => migrate_database(&database_url),
        OrchardCommand::ImportLegacyOrchard { geojson_path } => {
            import_orchard(&database_url, &geojson_path)
        }
        OrchardCommand::RunServer { address } => runserver(database_url, address),
    }
}

fn migrate_database(database_url: &str) -> ExitCode {
    let mut migrator = match PostgresMigrator::connect(database_url) {
        Ok(migrator) => migrator,
        Err(error) => {
            eprintln!("Migration failed: {error}.");
            return ExitCode::from(1);
        }
    };
    match migrator.migrate() {
        Ok(report) => {
            if report.adopted_legacy_schema {
                println!("Adopted the existing version-10 schema.");
            }
            if report.applied_versions.is_empty() {
                println!("Database schema is up to date.");
            } else {
                let versions = report
                    .applied_versions
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("Applied migrations: {versions}.");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Migration failed: {error}.");
            ExitCode::from(1)
        }
    }
}

fn import_orchard(database_url: &str, geojson_path: &std::path::Path) -> ExitCode {
    let mut orchard_storage = match PostgresOrchardStorage::connect(database_url) {
        Ok(orchard_storage) => orchard_storage,
        Err(_) => {
            eprintln!("Import failed: could not connect to the orchard database.");
            return ExitCode::from(1);
        }
    };
    match import_legacy_geojson_file(geojson_path, &mut orchard_storage) {
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

fn runserver(database_url: String, address: std::net::SocketAddr) -> ExitCode {
    let orchard_storage = match PostgresOrchardStorage::connect(&database_url) {
        Ok(orchard_storage) => orchard_storage,
        Err(_) => {
            eprintln!("Server failed: could not connect to the orchard database.");
            return ExitCode::from(1);
        }
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => {
            eprintln!("Server failed: could not start the HTTP runtime.");
            return ExitCode::from(1);
        }
    };

    runtime.block_on(async move {
        match start_http_server(orchard_storage, address).await {
            Ok(server) => {
                println!("Listening on {}", server.url());
                let _ = io::stdout().flush();
                match server.wait().await {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(()) => {
                        eprintln!("Server failed: the HTTP server stopped unexpectedly.");
                        ExitCode::from(1)
                    }
                }
            }
            Err(_) => {
                eprintln!("Server failed: could not bind the HTTP server address.");
                ExitCode::from(1)
            }
        }
    })
}
fn import_error_message(error: GeoJsonLegacyOrchardImportError) -> String {
    match error {
        GeoJsonLegacyOrchardImportError::CouldNotImportOrchard(
            LegacyOrchardImportError::LegacyFeatureAlreadyImported { legacy_feature_id },
        ) => {
            format!("legacy feature {legacy_feature_id} is already imported. No changes were made.")
        }
        GeoJsonLegacyOrchardImportError::CouldNotReadGeoJson => {
            "could not read the legacy GeoJSON file.".into()
        }
        GeoJsonLegacyOrchardImportError::CouldNotParseGeoJson => {
            "could not parse the legacy GeoJSON file.".into()
        }
        GeoJsonLegacyOrchardImportError::CouldNotParsePlantIdentity { legacy_feature_id } => {
            format!("could not parse plant identity for legacy feature {legacy_feature_id}.")
        }
        GeoJsonLegacyOrchardImportError::CouldNotParseHarvestWindow { legacy_feature_id } => {
            format!("could not parse harvest window for legacy feature {legacy_feature_id}.")
        }
        GeoJsonLegacyOrchardImportError::CouldNotImportOrchard(_) => {
            "the orchard could not be imported. No changes were made.".into()
        }
    }
}
