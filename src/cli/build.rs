use clap::Args;
use tracing::{
    error,
    info,
};

use super::CommandError;
use crate::{
    imply_all,
    package::{
        Package,
        build::BuildError,
    },
};

/// Build a package from source
#[derive(Args, Debug)]
pub struct Command {
    /// Package name, optionally with the version
    #[arg(value_name = "PACKAGE", num_args = 1..)]
    pub packages: Vec<String>,

    /// Whether to forcibly build a package
    #[arg(long, short)]
    pub force: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        for pkg in &pkgs {
            match pkg.build(self.force) {
                | Err(BuildError::ShouldntBuild) => {
                    info!(
                        "Not rebuilding {pkg:-}, pass --force or edit its pkgfile to force a rebuild."
                    );
                    println!(
                        "Not rebuilding {pkg:-}, pass --force or edit its pkgfile to force a rebuild."
                    );
                },
                | Err(e) => {
                    error!("Failed to build {pkg:-}: {e}");
                    eprintln!("Failed to build {pkg:-}: {e}");
                    return Err(CommandError::from(e))
                },
                | Ok(_) => {
                    info!("Built {pkg:-}");
                    println!("Built {pkg:-}");
                },
            }
        }

        Ok(())
    }
}
