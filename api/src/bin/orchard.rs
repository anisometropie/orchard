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
use orchard_api::hexagon::use_cases::set_user_password::{
    UserPasswordChangeError, UserPasswordChangeRequested, set_user_password,
};

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
        OrchardCommand::RevertMigrations { target_version } => {
            revert_migrations(&database_url, target_version)
        }
        OrchardCommand::ImportLegacyOrchard { geojson_path } => {
            import_orchard(&database_url, &geojson_path)
        }
        OrchardCommand::RunServer { address } => runserver(database_url, address),
        OrchardCommand::SetUserPassword { username } => {
            set_password_from_environment(&database_url, username)
        }
    }
}

fn set_password_from_environment(database_url: &str, username: String) -> ExitCode {
    let password = match env::var("ORCHARD_USER_PASSWORD") {
        Ok(password) => password,
        Err(_) => {
            eprintln!("Password change failed: ORCHARD_USER_PASSWORD is not configured.");
            return ExitCode::from(2);
        }
    };
    let mut access_control = match PostgresOrchardStorage::connect(database_url) {
        Ok(storage) => storage,
        Err(_) => {
            eprintln!("Password change failed: could not connect to the orchard database.");
            return ExitCode::from(1);
        }
    };
    match set_user_password(
        UserPasswordChangeRequested {
            username: username.clone(),
            new_password: password,
        },
        &mut access_control,
    ) {
        Ok(()) => {
            println!("Password changed for {username}.");
            ExitCode::SUCCESS
        }
        Err(UserPasswordChangeError::PasswordTooShort) => {
            eprintln!("Password change failed: password must contain at least 12 characters.");
            ExitCode::from(2)
        }
        Err(UserPasswordChangeError::UserNotFound) => {
            eprintln!("Password change failed: user {username} was not found.");
            ExitCode::from(1)
        }
        Err(UserPasswordChangeError::PasswordCouldNotBeChanged) => {
            eprintln!("Password change failed: the database could not be updated.");
            ExitCode::from(1)
        }
    }
}

fn revert_migrations(database_url: &str, target_version: u32) -> ExitCode {
    let mut migrator = match PostgresMigrator::connect(database_url) {
        Ok(migrator) => migrator,
        Err(error) => {
            eprintln!("Migration revert failed: {error}.");
            return ExitCode::from(1);
        }
    };
    match migrator.revert_to(target_version) {
        Ok(report) if report.reverted_versions.is_empty() => {
            println!("Database schema is already at or below migration {target_version}.");
            ExitCode::SUCCESS
        }
        Ok(report) => {
            let versions = report
                .reverted_versions
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            println!("Reverted migrations: {versions}.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Migration revert failed: {error}.");
            ExitCode::from(1)
        }
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
