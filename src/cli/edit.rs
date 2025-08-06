use clap::Args;
use tracing::{
    error,
    info,
};

use crate::exec_interactive;
use color_eyre::eyre::{bail, Result as Eresult};

/// Edit build instructions and/or metadata for a package
#[derive(Args, Debug)]
pub struct Command {
    /// The packages to edit
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Skip building
    #[arg(long, short)]
    pub skip_build: bool,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        for pkg in &self.packages {
            let name = pkg.split_once('@').map(|(n, _)| n).unwrap_or(pkg);

            if exec_interactive!(
                "SKIP_BUILD={} {}/edit-package {name}",
                self.skip_build,
                super::SCRIPT_DIR,
            )
            .is_err()
            {
                error!("Failed to edit {name}");
                bail!("Failed to edit {name}");
            }

            info!("Edited {name}");
        }
        Ok(())
    }
}
