// package/alias.rs
//! Functions for handling package aliases

use std::{
    fs::{
        self,
        DirEntry,
    },
    path::{
        Path,
        PathBuf,
    },
};

use tracing::{
    error,
    instrument,
    trace,
};

use super::Package;
use crate::package::FormError;

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
}

#[cfg(debug_assertions)]
impl PartialEq for Alias {
    fn eq(&self, other: &Alias) -> bool { self.name == other.name }
}

impl From<PathBuf> for Alias {
    fn from(value: PathBuf) -> Self {
        Self::new(
            value
                .file_name()
                .expect("Alias should have a filename")
                .to_string_lossy(),
        )
    }
}

impl From<DirEntry> for Alias {
    fn from(value: DirEntry) -> Self { Self::new(value.file_name().to_string_lossy()) }
}

impl TryFrom<Alias> for Package {
    type Error = FormError;

    fn try_from(value: Alias) -> Result<Self, Self::Error> { Self::from_s_file(&value.name) }
}

impl TryFrom<&Alias> for Package {
    type Error = FormError;

    fn try_from(value: &Alias) -> Result<Self, Self::Error> { Self::from_s_file(&value.name) }
}

impl Alias {
    pub fn new<S: Into<String>>(s: S) -> Self { Alias { name: s.into() } }

    pub fn original(&self) -> Result<Package, FormError> { self.try_into() }
}

#[memoize::memoize]
pub fn gather_all_aliases() -> Vec<Alias> {
    fs::read_dir("/var/db/to/pkgs")
        .inspect_err(|e| error!("Failed to read pkg database: {e}"))
        .expect("Should be able to read pkg database")
        .map_while(Result::ok)
        .filter(|e| e.path().is_symlink())
        .map(|e| e.into())
        .collect()
}

impl Package {
    pub fn find_aliases(&self) -> Vec<Alias> {
        gather_all_aliases()
            .iter()
            .filter(|a| {
                a.original()
                    .inspect_err(|e| {
                        error!("Failed to find original package for alias '{a:?}': {e}")
                    })
                    .map(|a| a.name == *self.name) // don't match the whole struct, because depkind
                    .unwrap_or(false)
            })
            .inspect(|a| trace!("Found alias '{a:?}' for {self:-}"))
            .cloned()
            .collect::<Vec<_>>()
    }

    #[instrument(level = "trace")]
    pub fn alias_pkgdirs(&self) -> Vec<PathBuf> {
        self.find_aliases()
            .iter()
            .map(|a| Path::new("/var/db/to/pkgs").join(&a.name))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::path::{
        Path,
        PathBuf,
    };

    use permitit::Permit;

    use super::{
        super::Package,
        Alias,
    };
    use crate::package::dep::DepKind;

    #[test]
    fn ogg_alias() {
        let pkg = Package::from_s_file("ogg").unwrap();

        assert_eq!(vec![Alias::new("libogg")], pkg.find_aliases());
    }

    #[test]
    fn ogg_pkgdir() {
        let pkg = Package::from_s_file("ogg").unwrap();

        assert_eq!(
            vec![PathBuf::from("/var/db/to/pkgs/libogg")],
            pkg.alias_pkgdirs()
        )
    }

    #[test]
    fn ogg_pkgdir_depkind() {
        let mut pkg = Package::from_s_file("ogg").unwrap();
        pkg.depkind = Some(DepKind::Required);

        assert_eq!(
            vec![PathBuf::from("/var/db/to/pkgs/libogg")],
            pkg.alias_pkgdirs()
        )
    }

    #[ignore = "intrusive"]
    #[test]
    fn retain_alias_structure() {
        let pkg = Package::from_s_file("ogg").unwrap();
        const MERGED: &str = "/var/lib/to/chroot/merged";

        for alias_path in pkg.alias_pkgdirs() {
            let symlink = Path::new(MERGED).join(alias_path.strip_prefix("/").unwrap());
            eprintln!("Symlink: {symlink:?}");
            assert!(
                std::os::unix::fs::symlink(&pkg.name, &symlink)
                    .permit_if(
                        symlink.is_symlink()
                            && symlink.read_link().unwrap() == PathBuf::from("ogg")
                    )
                    .inspect_err(|e| eprintln!("{e}"))
                    .is_ok()
            );
            assert!(symlink.exists());
            assert!(symlink.is_symlink());
            assert_eq!(symlink.read_link().unwrap(), PathBuf::from("ogg"));
        }
    }

    #[test]
    fn pam_alias() {
        let pkg = Package::from_s_file("pam").unwrap();

        assert_eq!(vec![Alias::new("pam")], pkg.find_aliases());
    }
}
