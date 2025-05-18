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
    path::{
        Path,
        PathBuf,
    },
};

use anyhow::{
    Context,
    Result,
    bail,
};
use fshelpers::rm;
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
fn read_all_manifests(manifests: &[PathBuf]) -> Result<HashMap<PathBuf, Vec<String>>> {
    let mut data = HashMap::new();

    for manifest in manifests {
        let contents = read_to_string(manifest)
            .with_context(|| format!("Failed to open manifest '{}'", manifest.display()))?;
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
) -> Result<Vec<String>> {
    debug!("Finding unique files for {this_manifest:?}");

    error!("[FIXME] ALL DATA: {all_data:#?}");
    assert!(all_data.contains_key(this_manifest));
    let this_data = all_data.get(this_manifest).context("Missing manifest")?;
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
pub fn find_unique_paths(manifest: &PathBuf) -> Result<Vec<String>> {
    // Here a depth of 2 is used because the we are in the data directory for all packages
    let manifests = locate("/var/db/to/data", 2);
    let data = read_all_manifests(&manifests)?;
    find_unique(&data, manifest)
}

impl Package {
    pub fn manifest(&self) -> Option<PathBuf> {
        self.installed_version()
            .map(|iv| self.datadir().join(format!("MANIFEST@{iv}")))
    }

    pub fn remove(&self, force: bool, remove_critical: bool) -> Result<()> {
        if !self.is_installed() && !force {
            warn!("Can't remove {self} as it's not installed");
            bail!("Not installed")
        }

        if self.tags.iter().any(|t| t == "critical") && !remove_critical {
            warn!("Not removing {self} as it's tagged as critical");
            warn!("To force removal, pass --im-really-fucking-stupid");
            bail!("Critical")
        }

        if self.tags.iter().any(|t| t == "core") && !force {
            warn!("Not removing {self} as it's tagged as core");
            warn!("To force removal, pass --force");
            bail!("Core")
        }

        let manifest = self.manifest().with_context(|| {
            format!("[UNREACHABLE] Package {self} isn't installed but we got here somehow?")
        })?;

        let Ok(unique) = find_unique_paths(&manifest)
            .inspect_err(|e| error!("Some really weird manifest fuckery is happening. You shouldn't be seeing this error: {e}"))
        else {
            bail!("Unexpected")
        };

        debug!("Removing paths unique to {self}: {unique:#?}");
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

        // This should not fail
        rm(self.datadir().join("IV")).with_context(|| format!("Failed to remove IV for {self}"))?;

        // TODO: Add flags and configure options for removing dists and sources

        self.message(MessageHook::Remove);
        Ok(())
    }

    #[instrument]
    pub fn remove_dead_files_after_update(&self) -> Result<()> {
        if !self.is_installed() {
            error!(
                "[UNREACHABLE] Not removing files after update for {self} as it's not installed"
            );
            bail!("Not installed");
        }

        let dead_files = find_dead_files(self)?;
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
#[instrument]
pub fn find_dead_files(package: &Package) -> Result<Vec<String>> {
    if !package.is_installed() {
        warn!("Attempted to find dead files for uninstalled package '{package}'");
        bail!("Not installed");
    }

    // Read all manifests for the current package
    // Here a depth of 1 is used because the we are in the subdirectory for a specific package in
    // the data directory
    let manifests = locate(package.datadir(), 1);
    let data = read_all_manifests(&manifests)?;

    let this_manifest = package
        .manifest()
        .with_context(|| format!("Failed to find current manifest for {package}"))?;

    find_unique(&data, &this_manifest)
}
