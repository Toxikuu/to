use clap::Args;
use tracing::{
    error,
    info,
};

use crate::exec_interactive;
use color_eyre::eyre::{bail, Result as Eresult};

/// Delete a package from the package repository
#[derive(Args, Debug)]
pub struct Command {
    /// The packages to delete
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        for pkg in &self.packages {
            let name = pkg.split_once('@').map(|(n, _)| n).unwrap_or(pkg);

            if exec_interactive!("{}/delete-package {name}", super::SCRIPT_DIR).is_err() {
                error!("Failed to delete {pkg}");
                bail!("Failed to delete {pkg}");
            }

            info!("Deleted {pkg}");
        }

        Ok(())
    }
}
