// utils/file.rs
//! File-related utilities

// This is kinda fucking stupid but I really don't have a better way to check just given a string
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

pub fn is_download(s: &str) -> bool {
    let filename = s.rsplit_once("/").unwrap().1;
    filename.contains(".") && !filename.ends_with(".git")
}
