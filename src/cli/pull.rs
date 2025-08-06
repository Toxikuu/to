use clap::Args;
use tracing::error;

use color_eyre::Result as Eresult;
use crate::{
    imply_all,
    package::{
        Package,
        pull::multipull,
    },
};

/// Pull a package's distfile from the server
#[derive(Args, Debug)]
pub struct Command {
    /// The package to install
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,

    /// Whether to forcibly pull
    #[arg(short, long)]
    pub force: bool,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        multipull(&pkgs, self.force)
            .await
            .inspect_err(|e| error!("Failed to pull one or more packages: {e}"))?;

        Ok(())
    }
}
