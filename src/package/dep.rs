// package/dep.rs

use std::{
    collections::HashSet,
    fmt,
};

use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    debug,
    error,
    instrument,
    trace,
};

use super::{
    FormError,
    Package,
};
use crate::{
    package::{
        all_package_names,
        install::in_build_environment,
    },
    utils::parse::us_array,
};

pub fn parse_deps(raw: &str) -> Vec<Dep> {
    us_array(raw).iter().map(|s| Dep::from_string(s)).collect()
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Hash)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy, Hash)]
// NOTE: Doc dependency support has been dropped as I'd rather just include them as make
// dependencies for the packages for which I want documentation.
pub enum DepKind {
    Required,
    Runtime,
    Build,
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Required => write!(f, "Required"),
            | Self::Runtime => write!(f, "Runtime"),
            | Self::Build => write!(f, "Build"),
        }
    }
}

impl Dep {
    pub fn from_string(str: &str) -> Self {
        if let Some((kind, str)) = str.split_once(',') {
            let kind = match kind {
                | "b" => DepKind::Build,
                | "r" => DepKind::Runtime,
                | _ => panic!("Unknown dep kind: {kind}"),
            };

            Self { name: str.to_string(), kind }
        } else {
            Self {
                name: str.to_string(),
                kind: DepKind::Required,
            }
        }
    }

    /// # Convert a dependency to a package
    ///
    /// This function does not sacrifice dependency data. `DepKind` is added as a field to
    /// `Package`.
    pub fn to_package(&self) -> Result<Package, FormError> {
        let mut pkg = Package::from_s_file(&self.name)?;
        pkg.depkind = Some(self.kind);
        Ok(pkg)
    }
}

// TODO: Refactoring idea: Add a DepFilter trait. Impl it for bool, DepKind, and closures. This
// would allow calling `resolve_deps()` like any of:
// * `resolve_deps(true)`
// * `resolve_deps(DepKind::Required)`
// * `resolve_deps(|k| !matches!(k, DepKind::Runtime)`
// Not certain if I want to do this because it would in ways simplify and complicate the API, while
// making this codebase even messier.
impl Package {
    #[cfg(test)]
    pub fn to_dep(&self) -> Result<Dep, FormError> {
        Ok(Dep {
            name: self.name.clone(),
            kind: self
                .depkind
                .ok_or(FormError::MissingMetadata("depkind".to_owned()))?,
        })
    }

    /// # Find shallow dependants for a package
    ///
    /// This function gathers all packages, and checks to see if their shallow dependencies contain
    /// `self.name`.
    ///
    /// # Errors
    /// - Will fail if `self` could not be converted to a `Dep`
    /// - Will fail if any package could not be formed
    pub fn dependants(&self) -> Result<Vec<Package>, FormError> {
        let all_packages = all_package_names()
            .iter()
            .map(|p| Package::from_s_file(p).inspect_err(|e| error!("Failed to form {p}: {e}")))
            .collect::<Result<Vec<_>, _>>()?;

        let mut dependants = Vec::new();
        for package in all_packages {
            if package.dependencies.iter().any(|d| d.name == self.name) {
                dependants.push(package)
            }
        }

        Ok(dependants)
    }

    /// # Finds deep dependencies for a package, with a filter
    ///
    /// This function serves as the backend for `resolve_deps()` and should not be called on its
    /// own.
    ///
    /// # Arguments
    /// * `resolved`        - A collection of already-resolved dependencies
    /// * `seen`            - A collection of packages whose dependencies have been resolved
    /// * `order`           - The dependency resolution order
    /// * `filter`          - The filter to apply to dependency resolution
    ///
    /// # Returns
    /// Mutates the `resolved` argument.
    #[instrument(skip(self, resolved, seen, order, filter), level = "trace")]
    fn deep_deps(
        &self,
        resolved: &mut HashSet<Dep>,
        seen: &mut HashSet<Package>,
        order: &mut Vec<Package>,
        filter: impl Fn(DepKind) -> bool + Copy,
    ) {
        for dep in &self.dependencies {
            if !filter(dep.kind) {
                trace!("Skipping {dep} per filter");
                continue;
            }

            if resolved.insert(dep.clone()) {
                dep.to_package()
                    .expect("Failed to form dependency package")
                    .deep_deps(resolved, seen, order, filter);
            }
        }

        // Avoid resolving dependencies for packages we've already resolved
        if seen.insert(self.clone()) {
            order.push(self.clone());
        }
    }

    /// # Resolves deep dependencies, with a filter
    ///
    /// This function wraps `deep_deps()`, and ensures the package is not a dependency of itself
    /// (`find -mindepth 1` kinda filtering).
    ///
    /// # Arguments
    /// * `filter`          - The filter to apply to dependecy resolution
    ///
    /// # Returns
    /// Notably returns a `Vec<Package>` instead of `Vec<Dep>`. This is done for reasons; ifykyk.
    ///
    /// # Examples
    /// ```rust
    /// // Resolve all deps except runtime deps
    /// let most_deps = pkg.resolve_deps(|k| !matches!(k, DepKind::Runtime));
    ///
    /// // Resolve all deps
    /// // A filter
    /// let all_deps = pkg.resolve_deps(|_| true);
    /// ```
    #[instrument(skip(self, filter), level = "debug")]
    pub fn resolve_deps(&self, filter: impl Fn(DepKind) -> bool + Copy) -> Vec<Package> {
        if self.dependencies.is_empty() {
            debug!("No dependencies for {self:-}");
            return vec![]
        }

        debug!("Resolving dependencies for {self:-}");
        let mut resolved = HashSet::new();
        let mut seen = HashSet::new();
        let mut order = Vec::new();

        self.deep_deps(&mut resolved, &mut seen, &mut order, filter);
        order.retain(|d| d.name != self.name);

        trace!("Resolved dependencies for {self:-}:");
        for dep in &order {
            trace!(" - {dep} ({})", dep.depkind.unwrap());
        }

        order
    }

    /// # Collects all dependencies that should be in the build chroot
    ///
    /// The idea is to first collect deep required dependencies, then shallow build dependencies,
    /// then the deep required dependencies for those shallow build dependencies. Duplicates are
    /// filtered out by name, then collected into a final `Vec`.
    ///
    /// # Errors
    /// - Will fail if a dependency could not be converted to a package
    #[instrument(skip(self))]
    pub fn collect_chroot_deps(&self) -> Result<Vec<Package>, FormError> {
        // Collect deep required dependencies
        let mut deps = self
            .resolve_deps(|k| matches!(k, DepKind::Required))
            .into_iter()
            .collect::<HashSet<_>>();

        // Collect build dependencies, as long as they're not already accounted for by `deps`
        let build_deps = self
            .dependencies
            .iter()
            .filter(|d| d.kind == DepKind::Build && !deps.iter().any(|dep| dep.name == d.name))
            .map(|d| d.to_package())
            .collect::<Result<Vec<_>, _>>()?;

        // Collect deep required dependencies for shallow build dependencies, as long as they're
        // not already accounted for by `deps`
        let deep_build_deps = build_deps
            .iter()
            .flat_map(|d| d.resolve_deps(|k| matches!(k, DepKind::Required)))
            .filter(|d| !deps.contains(d))
            .collect::<Vec<_>>();

        // Tack on extra dependencies
        deps.extend(build_deps);
        deps.extend(deep_build_deps);

        Ok(deps.iter().cloned().collect::<Vec<_>>())
    }

    /// # Collects all dependencies that should be installed
    pub fn collect_install_deps(&self) -> Vec<Package> {
        if in_build_environment() {
            self.resolve_deps(|k| matches!(k, DepKind::Required))
        } else {
            self.resolve_deps(|k| matches!(k, DepKind::Required | DepKind::Runtime))
        }
    }
}

impl fmt::Display for Dep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.sign_plus() {
            write!(f, "{} ({})", self.name, self.kind)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::package::{
        Package,
        dep::DepKind,
    };

    #[test]
    /// Confirm the dependencies to be installed for dbus are sane. This tests
    /// `collect_install_deps()`.
    fn dbus_install_deps() {
        let pkg = Package::from_s_file("dbus").unwrap();

        let expected = ["glibc", "expat", "blfs-bootscripts"]
            .iter()
            .map(|d| d.to_owned())
            .collect::<Vec<_>>();

        let all_deps = pkg
            .collect_install_deps()
            .iter()
            .map(|d| d.name.clone())
            .collect::<Vec<_>>();

        assert_eq!(expected, all_deps);
    }

    #[test]
    /// Confirm the dependencies to be installed to the chroot for dbus are sane. This tests
    /// `collect_chroot_deps()`.
    fn dbus_chroot_deps() {
        let pkg = Package::from_s_file("dbus").unwrap();

        let mut expected = [
            "glibc",
            "bzip2",
            "expat",
            "gdbm",
            "libffi",
            "libtirpc",
            "libnsl",
            "libxcrypt",
            "mpdecimal",
            "ncurses",
            "zlib",
            "zstd",
            "xz",
            "libelf",
            "binutils",
            "gmp",
            "mpfr",
            "mpc",
            "isl",
            "gcc",
            "perl",
            "openssl",
            "readline",
            "sqlite",
            "tzdata",
            "python",
            "flit-core",
            "packaging",
            "wheel",
            "setuptools",
            "meson",
            "samurai",
            "blfs-bootscripts",
        ]
        .iter()
        .map(|d| d.to_owned())
        .collect::<Vec<_>>();

        let mut observed = pkg
            .collect_chroot_deps()
            .unwrap()
            .iter()
            .map(|d| d.name.clone())
            .collect::<Vec<_>>();

        observed.sort_unstable();
        expected.sort_unstable();

        assert_eq!(expected, observed);
    }

    /// Confirm make-ca isn't a dependency of itself, as it once wanted to be.
    #[test]
    fn make_ca_runtime_of_self() {
        let pkg = Package::from_s_file("make-ca").unwrap();

        let all_deps = pkg.resolve_deps(|_| true); // pull all dependencies
        dbg!(&all_deps);

        assert!(all_deps.iter().all(|d| d.name != "make-ca"))
    }

    /// Elogind depends on polkit as a runtime dependency. Polkit has glib listed as a required
    /// dependency. Confirm that glib is no longer pulled in.
    #[test]
    fn elogind_runtime_required() {
        let pkg = Package::from_s_file("elogind").unwrap();

        let all_deps = pkg.resolve_deps(|k| !matches!(k, DepKind::Runtime));
        dbg!(&all_deps);

        assert!(all_deps.iter().any(|d| d.name == "acl"));
        assert!(all_deps.iter().all(|d| d.name != "polkit")); // test shallow filtering
        assert!(all_deps.iter().all(|d| d.name != "glib")); // test deep filtering
    }

    /// Vala depends on libx11 at build time. This test ensures util-macros, a required dependency
    /// of libx11, is pulled in. This test also mimics the dependency resolution in
    /// `crate::package::build`.
    #[test]
    fn vala_build_deep() {
        let pkg = Package::from_s_file("vala").unwrap();

        let mut deps = pkg
            .resolve_deps(|k| matches!(k, DepKind::Required))
            .into_iter()
            .collect::<HashSet<_>>();

        eprintln!("\n\nBefore adding shallow build dependencies:");
        for dep in &deps {
            eprintln!("{:+}", dep.to_dep().unwrap())
        }

        let build_deps = pkg
            .dependencies
            .iter()
            .filter(|d| d.kind == DepKind::Build)
            .map(|d| d.to_package().unwrap())
            .collect::<Vec<_>>();

        let deep_build_deps = build_deps
            .iter()
            .flat_map(|d| d.resolve_deps(|k| matches!(k, DepKind::Required)))
            .collect::<Vec<Package>>();

        deps.extend(build_deps);
        deps.extend(deep_build_deps);
        let deps = deps.iter().map(|d| d.to_dep().unwrap()).collect::<Vec<_>>();

        eprintln!("\n\nAfter adding shallow build dependencies and their required dependencies:");
        for dep in &deps {
            eprintln!("{dep:+}")
        }

        // Ensure libx11 is pulled in as a build dependency
        assert!(
            deps.iter()
                .any(|d| d.name == "libx11" && d.kind == DepKind::Build)
        );

        // Ensure util-macros was also resolved
        assert!(deps.iter().any(|d| d.name == "util-macros"));
    }

    #[test]
    /// Same idea as the previous test. Just test the sanity of `collect_chroot_deps()`.
    fn vala_chroot_deps() {
        let pkg = Package::from_s_file("vala").unwrap();

        let deps = pkg.collect_chroot_deps().unwrap();

        // Ensure libx11 is pulled in as a build dependency
        assert!(
            deps.iter()
                .any(|d| d.name == "libx11" && d.depkind.unwrap() == DepKind::Build)
        );

        // Ensure util-macros was also resolved
        assert!(deps.iter().any(|d| d.name == "util-macros"));
    }

    /// Since make-ca is both a runtime and a required dependency, some weird shit used to happen.
    /// Test to ensure that weird shit doesn't happen.
    ///
    /// By weird shit, I mean the dependency resolver used to only hash dependencies by name, not
    /// also by kind. This should be fixed now.
    #[test]
    fn rust_make_ca_dep() {
        let pkg = Package::from_s_file("rust").unwrap();

        let all_deps = pkg.resolve_deps(|_| true);
        eprintln!("Deps:");
        for dep in &all_deps {
            eprintln!("{:>16} ({})", dep.name, dep.depkind.unwrap())
        }

        assert!(
            all_deps
                .iter()
                .filter(|d| d.name == "make-ca")
                .collect::<Vec<_>>()
                .len()
                > 1
        );

        assert!(
            all_deps
                .iter()
                .any(|d| d.name == "make-ca" && d.depkind.unwrap() == DepKind::Required)
        );

        assert!(
            all_deps
                .iter()
                .any(|d| d.name == "make-ca" && d.depkind.unwrap() == DepKind::Runtime)
        );

        let deps = all_deps
            .into_iter()
            .filter(|d| d.depkind.expect("Dep should have a kind") != DepKind::Runtime)
            .collect::<Vec<_>>();

        assert!(deps.iter().any(|d| d.name == "make-ca"));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Required));
    }
}
