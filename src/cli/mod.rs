macro_rules! command_boilerplate {
    ( $( $Variant:ident ),* $(,)? ) => {
        paste::paste! {
            $(
                pub mod [<$Variant:lower>];
            )*

            #[derive(Debug, clap::Subcommand)]
            #[non_exhaustive]
            pub enum Command {
                $(
                    $Variant([<$Variant:lower>]::Command),
                )*
            }

            impl Cli {
                pub async fn run(&self) -> Eresult<()> {
                    match &self.command {
                        $(
                            | Command::$Variant(x) => x.run().await,
                        )*
                    }
                }
            }
        }
    };
}

pub const SCRIPT_DIR: &str = "/usr/share/to/scripts/maintainer";

use std::io;

use crate::utils::err::eyre_prelude::*;
use clap::Parser;
use thiserror::Error;

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("TO_VERSION"),
    author,
    about,
    infer_subcommands = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Invalid syntax")]
    InvalidSyntax,

    #[error("Missing dependency: {0}")]
    MissingDependency(String),
}

command_boilerplate! {
    Serve,
    Add,
    Alias,
    Build,
    Bump,
    Delete,
    Edit,
    Generate,
    Lint,
    Health,
    Push,
    Data,
    Install,
    Prune,
    Pull,
    Remove,
    Sync,
    View,
    Vf,
}

#[macro_export]
macro_rules! imply_all {
    ($args:expr) => {
        if $args.packages.is_empty() {
            $crate::package::all_package_names()
        } else {
            $args.packages.to_vec()
        }
    };
}
