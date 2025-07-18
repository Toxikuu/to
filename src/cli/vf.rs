use clap::Args;
use futures::future::join_all;
use tracing::error;

use super::CommandError;
use crate::{
    imply_all,
    package::Package,
};

/// Fetch the latest upstream version for a package
#[derive(Args, Debug)]
pub struct Command {
    /// The package(s) to vf
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,

    /// Only show outdated packages
    #[arg(long, short)]
    pub outdated_only: bool,

    /// Ignore the vf cache
    #[arg(long, short)]
    pub ignore_cache: bool,

    /// Reuse the vf cache
    #[arg(long, short)]
    pub reuse_cache: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        let tasks = pkgs
            .iter()
            .map(|p| {
                let p_clone = p.clone();
                let ic = self.ignore_cache;
                let rc = self.reuse_cache;
                tokio::spawn(async move { p_clone.vf(ic, rc).await })
            })
            .collect::<Vec<_>>();

        let mut vfs = Vec::new();
        for res in join_all(tasks).await {
            match res {
                | Ok(Ok(vf)) => vfs.push(vf),
                | Ok(_) => {},
                | Err(e) => {
                    error!("Task join error: {e}");
                },
            }
        }

        for vf in vfs {
            if vf.is_current && self.outdated_only {
                continue
            }

            vf.display();
        }

        Ok(())
    }
}
