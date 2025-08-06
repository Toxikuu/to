use clap::Args;
use color_eyre::eyre::bail;
use std::process::exit;
use tracing::{
    debug,
    error,
    info,
};

use crate::{
    package::{
        all_package_names, build::{get_build_order, BuildError}, Package
    },
};

use crate::utils::err::eyre_prelude::*;

/// Build a package from source
#[derive(Args, Debug)]
pub struct Command {
    /// Package name, optionally with the version
    ///
    /// If no packages are specified, every package is built. This differs from normal behavior in
    /// that the order in which all packages should be built is resolved as well.
    #[arg(value_name = "PACKAGE", num_args = 0..)]
    pub packages: Vec<String>,

    /// Forcibly build a package
    #[arg(long, short)]
    pub force: bool,

    /// Only output the build order
    ///
    /// This will dump the order in which all packages would be built if no packages are specified.
    #[arg(long, short = 'o')]
    pub dump_order: bool,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let pkgs = if self.packages.is_empty() {
            let all_packages = all_package_names()
                .iter()
                .map(|p| Package::from_s_file(p))
                .collect::<Result<_, _>>()?;
            get_build_order(all_packages)
        } else {
            self.packages.iter().map(|p| Package::from_s_file(p)).collect::<Result<_, _>>()?
        };

        if self.dump_order {
            debug!("Dumping build order to stdout");
            for p in &pkgs {
                println!("{p:-}");
            }
            exit(0);
        }

        info!("Building packages:");
        for p in &pkgs {
            info!(" - {p}");
        }

        for pkg in &pkgs {
            match pkg.build(self.force) {
                | Err(BuildError::ShouldntBuild) => {
                    info!(
                        "Not rebuilding {pkg:-}, pass --force or edit its pkgfile to force a rebuild."
                    );
                },
                | Err(e) => {
                    error!("Failed to build {pkg:-}: {e}");
                    bail!("Failed to build {pkg:-}");
                },
                | Ok(_) => {
                    info!("Built {pkg:-}");
                },
            }
        }

        Ok(())
    }
}
