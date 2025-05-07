// utils/parse.rs
//! Utilities for parsing things

/// # Parses a unit separator delimited array
/// The unit separator (\x1f) is used because white space is a shit IFS
/// Empty fields are removed
pub fn us_array(str: &str) -> Vec<String> {
    str.split('\x1f')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// # Checks if a given string looks like a git sha
pub fn is_commit_sha(s: &str) -> bool { s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()) }
