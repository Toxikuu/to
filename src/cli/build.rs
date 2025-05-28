use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    imply_all,
    package::Package,
};

#[derive(Args, Debug)]
pub struct Command {
    /// Package name, optionally with the version
    #[arg(value_name = "PACKAGE", num_args = 1..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        for pkg in &pkgs {
            pkg.build()
                .inspect_err(|e| error!("Failed to build {pkg}: {e}"))?;
        }

        Ok(())
    }
}
