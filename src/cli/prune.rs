use clap::Args;
use tracing::{
    info,
    warn,
};

use color_eyre::Result as Eresult;
use crate::{
    imply_all,
    package::Package,
};

// TODO: Improve pruning
/// Prune stale files for a package
#[derive(Args, Debug)]
pub struct Command {
    /// The package(s) to prune
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        for pkg in &pkgs {
            pkg.prune()
                .inspect_err(|e| warn!("Failed to prunue {pkg:-}: {e}"))?;

            info!("Pruned {pkg:-}")
        }

        Ok(())
    }
}
