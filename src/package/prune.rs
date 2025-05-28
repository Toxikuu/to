// package/prune.rs

use std::{
    fs::read_dir,
    io,
};

use fshelpers::rmr;
use thiserror::Error;
use tracing::{
    debug,
    info,
    trace,
    warn,
};

use super::Package;

#[derive(Error, Debug)]
pub enum PruneError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

impl Package {
    pub fn prune(&self) -> Result<(), PruneError> {
        let version = &self.version;
        let sources = &self
            .sources
            .iter()
            .map(|s| s.dest.clone())
            .collect::<Vec<_>>();
        debug!("Checking for pruneable sources");

        let sourcedir = self.sourcedir();
        let distdir = self.distdir();
        let datadir = self.datadir();

        let pruneable_sources = if sourcedir.exists() {
            read_dir(sourcedir)
                .inspect_err(|e| warn!("Failed to read sourcedir: {e}"))?
                .map_while(Result::ok)
                .filter(|f| !sources.contains(&f.file_name().to_string_lossy().to_string()))
                .map(|f| f.path())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        trace!("Found {pruneable_sources:?}");

        debug!("Checking for pruneable dists");
        let pruneable_dists = if distdir.exists() {
            read_dir(distdir)?
                .map_while(Result::ok)
                .filter(|f| {
                    !f.file_name()
                        .to_string_lossy()
                        .to_string()
                        .contains(version)
                })
                .map(|f| f.path())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        trace!("Found {pruneable_dists:?}");

        debug!("Checking for pruneable manifests");
        let pruneable_manifests = if datadir.exists() {
            read_dir(self.datadir())?
                .map_while(Result::ok)
                .filter(|f| {
                    let file_name = f.file_name().to_string_lossy().to_string();
                    // Skip files that don't end with version and that don't start with MANIFEST@
                    !file_name.ends_with(version) && !file_name.starts_with("MANIFEST@")
                })
                .map(|f| f.path())
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        trace!("Found {pruneable_manifests:?}");

        for f in pruneable_sources {
            trace!("Pruning source:   {}", f.display());
            if let Err(e) = rmr(&f) {
                warn!("Failed to prune source {}: {e}", f.display())
            }
        }
        // TODO: Test for safety
        for f in pruneable_dists {
            trace!("Pruning dist:     {}", f.display());
            // rmr(f);
        }
        // TODO: Test for safety
        for f in pruneable_manifests {
            trace!("Pruning manifest: {}", f.display());
            // rmr(f);
        }

        info!("Pruned {self}");

        Ok(())
    }
}
