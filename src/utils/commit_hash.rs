// utils/commit_hash.rs
//! Utilities related to git commit hashes

/// # Check whether a given string is a git commit hash.
///
/// A git commit hash is defined as 40 ascii hexdigit characters.
///
/// # Arguments
/// * `s`       - A potential git commit hash, usually a version string
///
/// # Errors
/// - Shouldn't error, but weird shit might happen with multibyte characters, though I'm pretty
///   sure this shouldn't matter because the second boolean expression would return false.
///
/// # Examples
/// ```rust
/// assert!(!is(
///     "hi mom, this is what a git commit hash doesn't look like"
/// ))
/// assert!(is("3e53eef5bff5e87804ba2f27f8d82d8f55b68d16"))
/// ```
fn is(s: &str) -> bool { s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) }

/// # Shorten a git commit hash to 8 characters, otherwise return the version.
///
/// This function checks whether the given string is a git commit hash, and if so, truncates it.
///
/// # Arguments
/// * `s`       - A version string that might be a git commit hash
///
/// # Examples
/// ```rust
/// assert_eq!(
///     "3e53eef5",
///     try_shorten("3e53eef5bff5e87804ba2f27f8d82d8f55b68d16")
/// );
/// assert_eq!("4.0.2", try_shorten("4.0.2"));
/// assert_eq!("20250225", try_shorten("20250225"));
/// ```
pub fn try_shorten(s: &str) -> &str { if is(s) { &s[..8] } else { s } }
