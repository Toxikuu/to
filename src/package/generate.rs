// package/generate.rs

use std::fs::write;

use tracing::{
    info,
    instrument,
};

use crate::package::Package;

impl Package {
    #[instrument(level = "debug")]
    pub fn generate(name: &str) {
        let pkg = super::Package::new(name);
        let s = serde_json::to_string_pretty(&pkg).expect("Failed to serialize");
        write(format!("/var/cache/to/pkgs/{name}/s"), s).unwrap();
        info!("Generated {pkg}");
    }
}
