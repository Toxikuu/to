use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    imply_all,
    package::{
        Package,
        pull::multipull,
    },
};

#[derive(Args, Debug)]
pub struct Command {
    /// The package to install
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        multipull(&pkgs)
            .await
            .inspect_err(|e| error!("Failed to pull one or more packages: {e}"))?;

        Ok(())
    }
}
