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
                pub async fn run(&self) -> Result<(), CommandError> {
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

pub const SCRIPT_DIR: &str = "/usr/share/to/scripts";

use std::io;

use anyhow::Result;
use clap::Parser;
use thiserror::Error;

use crate::{
    package::{
        FormError,
        build::BuildError,
        generate::GenerateError,
        install::InstallError,
        lint::LintError,
        prune::PruneError,
        pull::DownloadError,
        remove::RemoveError,
    },
    server::core::ServeError,
};

#[derive(Debug, Parser)]
#[command(name = "to", version, author, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to form package: {0}")]
    FormError(#[from] FormError),

    #[error("Failed to remove package: {0}")]
    RemoveError(#[from] RemoveError),

    #[error("Failed to pull package: {0}")]
    PullError(#[from] DownloadError),

    #[error("Failed to prune package: {0}")]
    PruneError(#[from] PruneError),

    #[error("Failed to build package: {0}")]
    BuildError(#[from] BuildError),

    #[error("Failed to generate package: {0}")]
    GenerateError(#[from] GenerateError),

    #[error("Failed to install package: {0}")]
    InstallError(#[from] InstallError),

    #[error("Failed to run server: {0}")]
    ServeError(#[from] ServeError),

    #[error("Lints failed: {0}")]
    Lints(#[from] LintError),

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
