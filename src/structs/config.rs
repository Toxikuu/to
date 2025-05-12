// structs/config.rs
//! Config code

use std::{
    fs,
    sync::LazyLock,
};

use serde::Deserialize;
use tracing::warn;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub log_level: String,
    pub strip:     bool,
    pub tests:     bool,
    pub jobs:      usize,
    pub cflags:    String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "debug".to_string(),
            strip:     true,
            tests:     false,
            jobs:      num_cpus::get(),
            cflags:    "-march=x86-64-v2 -O2 -pipe".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = "/etc/to/config.toml";

        // TODO: Maybe use a match for clarity
        // I don't like matches for error handling, but inspect_err can be kinda confusing since it
        // looks like the errors are being logged for the success case :shrug:
        let Ok(config_str) = fs::read_to_string(config_path).inspect_err(|e| {
            warn!("Failed to read config file {config_path}: {e}");
            warn!("The default config will be used");
        }) else {
            return Self::default();
        };

        toml::de::from_str(&config_str)
            .inspect_err(|e| {
                warn!("Invalid config: {e}");
                warn!("The default config will be used");
            })
            .unwrap_or_default()
    }
}
