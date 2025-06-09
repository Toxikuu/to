use clap::Args;
use tracing::info;

use super::CommandError;
use crate::{
    exec_interactive,
    package::Package,
};

#[derive(Args, Debug)]
pub struct Command {
    /// The packages to bump
    ///
    /// Can be name@new_version, otherwise new_version is guessed
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Skip building
    #[arg(long, short)]
    pub skip_build: bool,

    /// Bump without any interaction or notes
    #[arg(long, short)]
    pub auto: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        for pkg in &self.packages {
            let (name, oldv, newv) = if let Some((name, newv)) = pkg.split_once('@') {
                let pkg = Package::from_s_file(name)?;
                (pkg.name, pkg.version, newv.to_string())
            } else {
                let pkg = Package::from_s_file(pkg)?;
                (
                    pkg.name.clone(),
                    pkg.version.clone(),
                    // TODO: Don't unwrap
                    pkg.version_fetch(true).await.unwrap().unwrap_or_default(),
                )
            };

            exec_interactive!(
                "SKIP_BUILD={} CURR={oldv} NEW={newv} AUTO={} {}/bump-package {name}",
                self.skip_build,
                self.auto,
                super::SCRIPT_DIR,
            )?;

            info!("Bumped {name}@{oldv} -> {newv}")
        }
        Ok(())
    }
}
