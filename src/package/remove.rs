// structs/remove.rs
//! Package removal-related functions
// TODO: Split manifest stuff into manifest.rs

use std::{
    collections::{
        HashMap,
        HashSet,
    },
    fmt::Debug,
    fs::read_to_string,
    io::{
        self,
        ErrorKind,
    },
    path::{
        Path,
        PathBuf,
    },
};

use fshelpers::rm;
use thiserror::Error;
use tracing::{
    debug,
    error,
    instrument,
    trace,
    warn,
};
use walkdir::{
    DirEntry,
    WalkDir,
};

use super::{
    Package,
    message::MessageHook,
};

/// Paths that should never be removed, regardless what a manifest says
const KEPT: &[&str] = &[
    "/",
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/lib",
    "/lib32",
    "/opt",
    "/proc",
    "/root",
    "/run",
    "/sbin",
    "/sys",
    "/usr",
    "/usr/bin",
    "/usr/lib",
    "/usr/lib32",
    "/usr/libexec",
    "/var/ports",
    "/usr/share",
    "/usr/share/pkgconfig",
    "/var",
];

/// # Returns true if a directory entry is a manifest
fn is_manifest(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .file_name()
            .to_str()
            .unwrap_or("")
            .starts_with("MANIFEST@")
}

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_str().unwrap_or("").starts_with('.')
}

/// # Find all manifests
/// Ignores hidden entries and entries that aren't manifests
/// Dir is usually /var/db/to/data
#[instrument]
pub fn locate<P>(dir: P, depth: usize) -> Vec<PathBuf>
where
    P: AsRef<Path> + Debug,
{
    WalkDir::new(dir)
        .min_depth(depth)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file() && is_manifest(e) && e.path().with_file_name("IV").exists()
        })
        .map(DirEntry::into_path)
        .collect()
}

/// # Reads manifests and returns a hashmap of their paths and their contents
#[instrument]
fn read_all_manifests(manifests: &[PathBuf]) -> Result<HashMap<PathBuf, Vec<String>>, io::Error> {
    let mut data = HashMap::new();

    for manifest in manifests {
        let contents = read_to_string(manifest)?;
        let lines = contents.lines().map(ToString::to_string).collect();
        data.insert(manifest.clone(), lines);
    }

    Ok(data)
}

/// # Find lines representing package install paths unique to this manifest
/// Backend for `find_unique_paths()`
/// Returns the unique lines in reverse order (meaning /path/to/file is above /path/to)
#[instrument(skip(all_data))]
fn find_unique(
    all_data: &HashMap<PathBuf, Vec<String>>,
    this_manifest: &PathBuf,
) -> Result<Vec<String>, io::Error> {
    debug!("Finding unique files for {this_manifest:?}");

    // debug_assert!(all_data.contains_key(this_manifest));
    let this_data = all_data
        .get(this_manifest)
        .ok_or(io::Error::from(ErrorKind::NotFound))?;
    let all_other_lines = all_data
        .iter()
        .filter(|(path, _)| *path != this_manifest)
        .flat_map(|(_, lines)| lines.iter())
        .collect::<HashSet<_>>();

    Ok(this_data
        .iter()
        .filter(|l| !all_other_lines.contains(l))
        .map(|p| format!("/{p}"))
        .rev()
        .collect())
}

/// # Finds paths unique to a manifest
/// Also prefixes those paths with /
pub fn find_unique_paths(manifest: &PathBuf) -> Result<Vec<String>, io::Error> {
    // Here a depth of 2 is used because the we are in the data directory for all packages
    let manifests = locate("/var/db/to/data", 2);
    let data = read_all_manifests(&manifests)?;
    find_unique(&data, manifest)
}

#[derive(Debug, Error)]
pub enum RemoveError {
    #[error("Package is not installed")]
    NotInstalled,

    #[error("Package is critical")]
    Critical,

    #[error("Package is core")]
    Core,

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

impl Package {
    pub fn manifest(&self) -> Option<PathBuf> {
        self.installed_version()
            .map(|iv| self.datadir().join(format!("MANIFEST@{}", iv.srversion())))
    }

    pub fn remove(
        &self,
        force: bool,
        remove_critical: bool,
        suppress: bool,
    ) -> Result<(), RemoveError> {
        if !self.is_installed() && !force {
            warn!("Can't remove {self} as it's not installed");
            return Err(RemoveError::NotInstalled)
        }

        if self.tags.iter().any(|t| t == "critical") && !remove_critical {
            warn!("Not removing {self} as it's tagged as critical");
            warn!("To force removal, pass --im-really-fucking-stupid");
            return Err(RemoveError::Critical)
        }

        if self.tags.iter().any(|t| t == "core") && !force {
            warn!("Not removing {self} as it's tagged as core");
            warn!("To force removal, pass --force");
            return Err(RemoveError::Core)
        }

        // TODO: Use `ManifestError::MissingManifest`
        let manifest = self.manifest().ok_or(RemoveError::NotInstalled)?;

        let unique = match find_unique_paths(&manifest) {
            | Ok(u) => u,
            | Err(e) => {
                error!(
                    "Some really weird manifest fuckery is happening. You shouldn't be seeing this error: {e}"
                );
                panic!("Unexpected failure");
            },
        };

        // TODO: Add support for prer() here
        //       This serves as a pre-remove hook in the pkgfile

        trace!("Removing paths unique to {self}: {unique:#?}");
        unique.iter().for_each(|p| {
            let path = Path::new("/").join(p);

            if KEPT.iter().any(|&s| path.ends_with(s)) {
                debug!("Retaining protected path: '{}'", path.display());
                return;
            }

            if let Err(e) = rm(&path) {
                warn!("Failed to remove path '{}': {e}", path.display());
            }

            trace!("'{p}' -x");
        });

        // TODO: Add support for r() here
        //       This serves as a post-remove hook in the pkgfile

        // This should not fail
        rm(self.datadir().join("IV"))?;

        // TODO: Add flags and configure options for removing dists and sources

        self.message(suppress, MessageHook::Remove);
        Ok(())
    }

    // FIX: Finds newly installed files instead of actual dead files :sob:
    // TODO: Maybe fixed? ^
    #[instrument(skip(self))]
    pub fn remove_dead_files_after_update(&self) -> Result<(), RemoveError> {
        if !self.is_installed() {
            warn!(
                "Attempted to update '{self:-}' despite it not being installed. Kindly report this as a bug."
            );
            return Err(RemoveError::NotInstalled)
        }

        let dead_files = find_dead_files(self)?;
        debug!("Found dead files for {self:-}:\n{dead_files:#?}");
        dead_files.iter().for_each(|p| {
            let path = Path::new(p);

            if KEPT.iter().any(|&s| path.ends_with(s)) {
                debug!("Retaining protected path: '{}'", path.display());
                return;
            }

            // `rm()` ignores missing files, so no warnings will be issued for those
            if let Err(e) = rm(path) {
                warn!("Failed to remove '{}': {e}", path.display());
                return;
            }

            trace!("'{p}' -x");
        });

        Ok(())
    }
}

/// # Finds unique (dead) files in an old manifest
/// Locates all manifests specific to that package, matching against them for dead files
#[instrument(skip(package))]
pub fn find_dead_files(package: &Package) -> Result<Vec<String>, RemoveError> {
    trace!("Finding dead files for {package:-}");
    if !package.is_installed() {
        warn!(
            "Attempted to find dead files for uninstalled package '{package:-}'. Kindly report this as a bug."
        );
        return Err(RemoveError::NotInstalled)
    }

    // Read all manifests for the current package
    // Here a depth of 1 is used because the we are in the subdirectory for a specific package in
    // the data directory
    let manifests = locate(package.datadir(), 1);
    let data = read_all_manifests(&manifests)?;

    // Calculate the old manifest from the yet-unoverwritten IV
    let old_manifest = PathBuf::from(format!(
        "/var/db/to/data/{}/MANIFEST@{}",
        package.name,
        match package.installed_version() {
            | Some(v) => v.srversion(),
            | None => {
                error!("IV exists but manifest doesn't?");
                return Err(RemoveError::NotInstalled)
            },
        },
    ));

    Ok(find_unique(&data, &old_manifest)?)
}
