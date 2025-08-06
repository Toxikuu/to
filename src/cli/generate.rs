use clap::Args;
use color_eyre::Result as Eresult;

use crate::{
    imply_all,
    package::Package,
};

/// Serialize package metadata
#[derive(Args, Debug)]
pub struct Command {
    /// The packages to generate
    #[arg(value_name = "PACKAGE", num_args = 1..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        let pkgs = imply_all!(self);

        for pkg in &pkgs {
            let name = pkg.split_once('@').map(|(n, _)| n).unwrap_or(pkg);
            Package::generate(name)?;
        }

        Ok(())
    }
}
