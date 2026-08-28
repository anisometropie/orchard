use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, PartialEq)]
pub enum OrchardCommand {
    RunServer { address: SocketAddr },
    ImportLegacyOrchard { geojson_path: PathBuf },
}

#[derive(Parser)]
#[command(name = "orchard")]
struct OrchardCli {
    #[command(subcommand)]
    command: OrchardCliCommand,
}

#[derive(Subcommand)]
enum OrchardCliCommand {
    #[command(name = "runserver")]
    RunServer {
        #[arg(long, default_value = "127.0.0.1:3000")]
        address: SocketAddr,
    },
    #[command(name = "import_legacy_orchard")]
    ImportLegacyOrchard { filename: PathBuf },
}

pub fn parse_command<I, T>(arguments: I) -> Result<OrchardCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = OrchardCli::try_parse_from(arguments)?;
    Ok(match cli.command {
        OrchardCliCommand::RunServer { address } => OrchardCommand::RunServer { address },
        OrchardCliCommand::ImportLegacyOrchard { filename } => {
            OrchardCommand::ImportLegacyOrchard {
                geojson_path: filename,
            }
        }
    })
}

#[cfg(test)]
mod orchard_cli_unit_test;
