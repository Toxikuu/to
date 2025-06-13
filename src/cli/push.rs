use std::{
    io::{
        self,
        ErrorKind,
    },
    time::SystemTime,
};

use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    config::CONFIG,
    exec,
    imply_all,
    package::{
        Package,
        pull::{
            create_client,
            get_local_modtime,
            get_upstream_modtime,
        },
    },
};

/// Push a package's distfile to the server
#[derive(Args, Debug)]
pub struct Command {
    /// The package to install
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,

    #[arg(long, short)]
    pub force: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        let client = create_client()
            .await
            .inspect_err(|e| error!("Failed to create client: {e}"))?;

        for pkg in &pkgs {
            let dist = pkg.distfile();
            let distfile = dist.display();
            let filename = dist
                .file_name()
                .ok_or(io::Error::from(ErrorKind::InvalidFilename))?
                .to_string_lossy();
            let addr = &CONFIG.server_address;
            let url = format!("{addr}/{filename}");

            let resp = client.get(&url).send().await?;

            let local_modtime = get_local_modtime(&dist).unwrap_or_else(SystemTime::now);
            let server_modtime =
                get_upstream_modtime(resp.headers()).unwrap_or(SystemTime::UNIX_EPOCH);

            let should_push = local_modtime > server_modtime;

            // Compare local modtime with server and only push if local is newer
            // TODO: Replace the curl with reqwest
            if (self.force || should_push)
                && exec!("curl --data-binary '@{distfile}' '{addr}/up/{filename}'").is_err()
            {
                error!("Failed to push {distfile} for {pkg} with curl")
            }
        }

        Ok(())
    }
}
