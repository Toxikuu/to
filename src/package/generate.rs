// package/generate.rs

use std::{
    fs::write,
    io,
};

use thiserror::Error;
use tracing::{
    info,
    instrument,
};

use crate::package::Package;

#[derive(Error, Debug)]
pub enum GenerateError {
    #[error("Failed to serialize package")]
    Serialization(#[from] serde_json::Error),

    #[error("Failed to write sfile")]
    WriteSfile(#[from] io::Error),
}

impl Package {
    #[instrument(level = "debug")]
    pub fn generate(name: &str) -> Result<(), GenerateError> {
        // TODO: Consider making new() return an error as well
        let pkg = super::Package::new(name);
        let s = serde_json::to_string_pretty(&pkg)?;
        write(format!("/var/db/to/pkgs/{name}/s"), s)?;

        info!("Generated {pkg}");
        Ok(())
    }
}
