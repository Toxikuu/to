// package/actions.rs
//! Code related to logging package actions (removal, installs, builds, edits)

// TODO: Documentation

use std::path::PathBuf;

use once_cell::sync::Lazy;
use tracing::warn;

use super::Package;
use crate::utils::file::{
    append,
    overwrite,
};

static CURRENT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/var/log/to/current"));
static ALLTIME: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/var/log/to/alltime"));

impl Package {
    pub fn log_installing(&self) {
        if let Err(e) = overwrite(&*CURRENT, format!("{self:<32} ::: INSTALL\n")) {
            warn!("Failed to log current action for {self}: {e}")
        }

        if let Err(e) = append(&*ALLTIME, format!("{self:<32} ::: INSTALL\n")) {
            warn!("Failed to log alltime action for {self}: {e}")
        }
    }

    pub fn log_removing(&self) {
        if let Err(e) = overwrite(&*ALLTIME, format!("{self:<32} ::: REMOVE\n")) {
            warn!("Failed to log action for {self}: {e}")
        }

        if let Err(e) = append(&*ALLTIME, format!("{self:<32} ::: REMOVE\n")) {
            warn!("Failed to log alltime action for {self}: {e}")
        }
    }

    pub fn log_building(&self) {
        if let Err(e) = overwrite(&*CURRENT, format!("{self:<32} ::: BUILD\n")) {
            warn!("Failed to log action for {self}: {e}")
        }

        if let Err(e) = append(&*ALLTIME, format!("{self:<32} ::: BUILD\n")) {
            warn!("Failed to log alltime action for {self}: {e}")
        }
    }

    pub fn log_editing(&self) {
        if let Err(e) = overwrite(&*CURRENT, format!("{self:<32} ::: EDIT\n")) {
            warn!("Failed to log action for {self}: {e}")
        }

        if let Err(e) = append(&*ALLTIME, format!("{self:<32} ::: EDIT\n")) {
            warn!("Failed to log alltime action for {self}: {e}")
        }
    }
}
