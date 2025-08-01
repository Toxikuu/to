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
    /// Log level (from trace to off, case insensitive)
    pub log_level:           String,
    /// Whether to log to the console
    pub log_to_console:      bool,
    /// Max log size in bytes
    pub log_max_size:        u64,
    /// Whether to run tests
    pub tests:               bool,
    /// Makeflags to use
    pub makeflags:           String,
    /// Stagefile to use for the build environment
    pub stagefile:           String,
    /// CFLAGS, CXXFLAGS, FFLAGS, and FCFLAGS to pass to the build environment
    pub cflags:              String,
    /// RUSTFLAGS to pass to the build environment
    pub rustflags:           String,
    /// Command used for `to view --tree <package>`
    pub tree_command:        String,
    /// Address of the distfileserver
    pub server_address:      String,
    /// URL for the package repository
    pub package_repo:        String,
    /// Branch for the package repository
    pub package_repo_branch: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level:           "debug".to_string(),
            log_to_console:      true,
            log_max_size:        64 * 1024 * 1024, // 64 MiB
            tests:               false,
            makeflags:           format!("-j{}", num_cpus::get()),
            stagefile:           "/usr/share/to/stagefile.tar.xz".to_string(),
            cflags:              "-march=x86-64-v3 -O2 -pipe".to_string(),
            rustflags:           "-C opt-level=2 -C target-cpu=x86-64-v3".to_string(),
            tree_command:        "tree -F".to_string(),
            server_address:      "127.0.0.1:7020".to_string(),
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
