// utils/file.rs
//! File-related utilities

use std::{
    fs::OpenOptions as OO,
    io::{
        self,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
    time::SystemTime,
};

use fshelpers::mkdir_p;

// This is kinda fucking stupid but I really don't have a better way to check just given a string.
// This is currently being kept as I may handle tarballs and zips in-house eventually.
// pub fn is_tarball(s: &str) -> bool {
//     s.to_ascii_lowercase().ends_with(".tar")
//         || s.ends_with(".tar.gz")
//         || s.ends_with(".tar.bz2")
//         || s.ends_with(".tar.xz")
//         || s.ends_with(".tar.zst")
//         || s.ends_with(".tar.lz")
//         || s.ends_with(".tar.lzma")
//         || s.ends_with(".tgz")
//         || s.ends_with(".tbz2")
//         || s.ends_with(".txz")
//         || s.ends_with(".tlz")
// }
//
// pub fn is_zip(s: &str) -> bool {
//     s.to_ascii_lowercase().ends_with(".zip")
// }

/// # Checks whether a given string is a SourceKind::Download
///
/// # Arguments
/// * `s`           - A url that may or may not be a downloadable file
///
/// # Errors
/// Panics if:
/// - The specified string doesn't contain '/' (urls should)
///
/// # Examples
/// ```rust
/// assert!(is_download("https://example.com/download.txt"));
/// assert!(!is_download("https://git.gay/gitgay/forgejo.git"));
/// ```
pub fn is_download(s: &str) -> bool {
    let filename = s.rsplit_once("/").unwrap().1;
    filename.contains(".") && !filename.ends_with(".git")
}

/// # Appends specified contents to a file, creating parent directories if needed.
///
/// You must explicitly specify new lines.
///
/// This function ensures the existence of the given file path by creating any missing parents. The
/// file is then created or opened, and the given contents are appended.
///
/// # Arguments
/// * `path`        - The file path to be overwritten.
/// * `contents`    - The data to write to the file.
///
/// # Errors
/// Returns an `io::Error` if:
/// - The parent directories could not be created.
/// - The file could not be opened.
/// - The file could not be written to.
///
/// # Examples
/// ```rust
/// append("output.txt", b"hi mom")?;
/// append("output.txt", "bye mom".to_string())?;
/// ```
pub fn append<P, C>(path: P, contents: C) -> io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        mkdir_p(parent)?;
    }

    let mut file = OO::new().create(true).append(true).open(path)?;
    file.write_all(contents.as_ref())
}

/// # Overwrites a file with the specified contents, creating parent directories if needed.
///
/// This function ensures the existence of the given file path by creating any missing parents. The
/// file is then created or truncated, and the given contents are written.
///
/// # Arguments
/// * `path`        - The file path to be overwritten.
/// * `contents`    - The data to write to the file.
///
/// # Errors
/// Returns an `io::Error` if:
/// - The parent directories could not be created.
/// - The file could not be opened.
/// - The file could not be written to.
///
/// # Examples
/// ```rust
/// overwrite("output.txt", b"hi mom")?;
/// overwrite("output.txt", "bye mom".to_string())?;
/// ```
pub fn overwrite<P, C>(path: P, contents: C) -> io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        mkdir_p(parent)?;
    }

    let mut file = OO::new().create(true).truncate(true).open(path)?;
    file.write_all(contents.as_ref())
}

/// # Check whether a program exists in $PATH.
///
/// This function uses the which crate to check the existence of a program.
///
/// # Arguments
/// * `program`     - The program to check.
///
/// # Examples
/// ```rust
/// if !exists("git") {
///     bail!("Missing program `git`")
/// }
///
/// if exists("tar") {
///     trace!("Program `tar` exists")
/// }
/// ```
#[inline]
// TODO: Move this somewhere more fitting. Organization is hard.
pub fn exists(program: &str) -> bool { which::which(program).is_ok() }

/// # Get a path's modtime
///
/// # Arguments
/// * `path`        - The path to check.
///
/// # Returns `None` if:
/// - The path doesn't exist, or could not be accessed
///
/// # Examples
/// ```rust
/// let modtime = mtime("/usr/bin/[").unwrap_or(SystemTime::UNIX_EPOCH); 
/// ```
pub fn mtime<P: AsRef<Path>>(path: P) -> Option<SystemTime> {
    path.as_ref()
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}
