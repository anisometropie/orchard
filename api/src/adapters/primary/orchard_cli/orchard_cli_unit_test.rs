use std::{net::SocketAddr, path::PathBuf};

use clap::error::ErrorKind;

use super::{OrchardCommand, parse_command};

#[test]
fn import_legacy_orchard() {
    assert_eq!(
        parse_command([
            "orchard",
            "import_legacy_orchard",
            "fixtures/legacy-orchard.geojson",
        ])
        .unwrap(),
        OrchardCommand::ImportLegacyOrchard {
            geojson_path: PathBuf::from("fixtures/legacy-orchard.geojson"),
        }
    );
}

#[test]
fn runserver() {
    assert_eq!(
        parse_command(["orchard", "runserver"]).unwrap(),
        OrchardCommand::RunServer {
            address: "127.0.0.1:3000".parse::<SocketAddr>().unwrap(),
        }
    );
}

#[test]
fn runserver_at_address() {
    assert_eq!(
        parse_command(["orchard", "runserver", "--address", "127.0.0.1:4567"]).unwrap(),
        OrchardCommand::RunServer {
            address: "127.0.0.1:4567".parse::<SocketAddr>().unwrap(),
        }
    );
}

#[test]
fn migrate_database() {
    assert_eq!(
        parse_command(["orchard", "migrate"]).unwrap(),
        OrchardCommand::Migrate,
    );
}

#[test]
fn revert_migrations_to_version() {
    assert_eq!(
        parse_command(["orchard", "migrate", "revert", "--to", "10"]).unwrap(),
        OrchardCommand::RevertMigrations { target_version: 10 },
    );
}

#[test]
fn require_filename() {
    assert_eq!(
        parse_command(["orchard", "import_legacy_orchard"])
            .unwrap_err()
            .kind(),
        ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn set_user_password_without_putting_the_password_in_process_arguments() {
    assert_eq!(
        parse_command(["orchard", "set_user_password", "--username", "alice"]).unwrap(),
        OrchardCommand::SetUserPassword {
            username: "alice".into(),
        }
    );
}
