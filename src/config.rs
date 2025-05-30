// structs/config.rs
//! Config code

// TODO: Once there are enough configure options, organize them into structs

use std::{
    fs,
    sync::LazyLock,
};

use serde::Deserialize;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub log_level:           String,
    pub strip:               bool,
    pub tests:               bool,
    pub makeflags:           String,
    pub stagefile:           String,
    pub cflags:              String,
    pub server_address:      String,
    pub package_repo:        String,
    pub package_repo_branch: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level:           "debug".to_string(),
            strip:               true,
            tests:               false,
            makeflags:           format!("-j{n} -l{n}", n = num_cpus::get()),
            cflags:              "-march=x86-64-v2 -O2 -pipe".to_string(),
            server_address:      "127.0.0.1:7020".to_string(),
            stagefile:           "/usr/share/to/stagefile.tar.xz".to_string(),
            package_repo:        "https://github.com/Toxikuu/to-pkgs.git".to_string(),
            package_repo_branch: "master".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = "/etc/to/config.toml";

        let config_str = match fs::read_to_string(config_path) {
            | Ok(c) => c,
            | Err(e) => {
                eprintln!("Failed to read config file at {config_path}: {e}");
                eprintln!("The default config will be used");
                return Self::default()
            },
        };

        match toml::de::from_str(&config_str) {
            | Ok(c) => c,
            | Err(e) => {
                eprintln!("\x1b[31;1mInvalid config: {e}\x1b[0m");
                eprintln!("\x1b[31;1mThe default config will be used\x1b[0m");
                Self::default()
            },
        }
    }
}
