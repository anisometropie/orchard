use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, PartialEq)]
pub enum OrchardCommand {
    Migrate,
    RevertMigrations { target_version: u32 },
    RunServer { address: SocketAddr },
    ImportLegacyOrchard { geojson_path: PathBuf },
    SetUserPassword { username: String },
}

#[derive(Parser)]
#[command(name = "orchard")]
struct OrchardCli {
    #[command(subcommand)]
    command: OrchardCliCommand,
}

#[derive(Subcommand)]
enum OrchardCliCommand {
    #[command(name = "migrate")]
    Migrate {
        #[command(subcommand)]
        command: Option<MigrationCliCommand>,
    },
    #[command(name = "runserver")]
    RunServer {
        #[arg(long, default_value = "127.0.0.1:3000")]
        address: SocketAddr,
    },
    #[command(name = "import_legacy_orchard")]
    ImportLegacyOrchard { filename: PathBuf },
    #[command(name = "set_user_password")]
    SetUserPassword {
        #[arg(long)]
        username: String,
    },
}

#[derive(Subcommand)]
enum MigrationCliCommand {
    Revert {
        #[arg(long = "to")]
        target_version: u32,
    },
}

pub fn parse_command<I, T>(arguments: I) -> Result<OrchardCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = OrchardCli::try_parse_from(arguments)?;
    Ok(match cli.command {
        OrchardCliCommand::Migrate { command: None } => OrchardCommand::Migrate,
        OrchardCliCommand::Migrate {
            command: Some(MigrationCliCommand::Revert { target_version }),
        } => OrchardCommand::RevertMigrations { target_version },
        OrchardCliCommand::RunServer { address } => OrchardCommand::RunServer { address },
        OrchardCliCommand::ImportLegacyOrchard { filename } => {
            OrchardCommand::ImportLegacyOrchard {
                geojson_path: filename,
            }
        }
        OrchardCliCommand::SetUserPassword { username } => {
            OrchardCommand::SetUserPassword { username }
        }
    })
}

#[cfg(test)]
mod orchard_cli_unit_test;
