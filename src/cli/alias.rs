use clap::Args;
use tracing::info;

use super::CommandError;
use crate::exec_interactive;
use color_eyre::eyre::{Report as Ereport, Result as Eresult, WrapErr};

/// Create an alias for a package
#[derive(Args, Debug)]
pub struct Command {
    /// Package name, optionally with the version
    #[arg(value_name = "PACKAGE", num_args = 2)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let from = self.packages.first().ok_or(CommandError::InvalidSyntax)?;
        let from = from.split_once('@').map_or(from.as_str(), |(n, _)| n);

        let to = self.packages.last().ok_or(CommandError::InvalidSyntax)?;
        let to = to.split_once('@').map_or(to.as_str(), |(n, _)| n);

        exec_interactive!("{}/alias-package {from} {to}", super::SCRIPT_DIR)?;
        info!("Created alias {to} for {from}");
        Ok(())
    }
}
