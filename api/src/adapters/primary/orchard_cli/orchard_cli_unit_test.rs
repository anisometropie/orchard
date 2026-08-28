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
fn require_filename() {
    assert_eq!(
        parse_command(["orchard", "import_legacy_orchard"])
            .unwrap_err()
            .kind(),
        ErrorKind::MissingRequiredArgument
    );
}
