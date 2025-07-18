// package/helpers.rs

use std::{
    fs::read_to_string,
    path::PathBuf,
};

use tracing::{error, instrument};

use super::{Package, Version};

impl Package {
    #[instrument(level = "debug")]
    pub fn installed_version(&self) -> Option<Version> {
        if self.is_installed() {
            read_to_string(self.datadir().join("IV"))
                .ok()
                .and_then(|s| s.trim().parse().inspect_err(|e| error!("Failed to parse IV for {self}: {e:?}")).ok())
        } else {
            None
        }
    }

    // PERF: Strong memoization candidate
    // Use once_cell's unsync::OnceCell and get_or_init() prolly
    pub fn distfile(&self) -> PathBuf { self.distdir().join(format!("{self}.tar.zst")) }

    // PERF: Strong memoization candidate
    pub fn pkgdir(&self) -> PathBuf { PathBuf::from("/var/db/to/pkgs").join(&self.name) }

    // PERF: Strong memoization candidate
    pub fn pkgfile(&self) -> PathBuf { self.pkgdir().join("pkg") }

    // PERF: Strong memoization candidate
    pub fn sfile(&self) -> PathBuf { self.pkgdir().join("s") }

    // PERF: Strong memoization candidate
    pub fn distdir(&self) -> PathBuf { PathBuf::from("/var/cache/to/dist").join(&self.name) }

    // PERF: Strong memoization candidate
    pub fn sourcedir(&self) -> PathBuf { PathBuf::from("/var/cache/to/sources").join(&self.name) }

    // PERF: Strong memoization candidate
    pub fn datadir(&self) -> PathBuf { PathBuf::from("/var/db/to/data").join(&self.name) }

    // PERF: Strong memoization candidate
    pub fn is_installed(&self) -> bool { self.datadir().join("IV").exists() }

    /// # Checks if a package is up to date, or current
    /// Returns false if the package is not installed
    pub fn is_current(&self) -> bool {
        let Some(iv) = self.installed_version() else {
            return false;
        };

        iv == self.version
    }
}
