// package/build.rs

use std::{
    process::exit,
    fs::{
        copy,
        write,
    },
    os::unix::fs,
    path::{
        Path,
        PathBuf,
    }
};

use fshelpers::{
    mkdir_p,
    mkf_p,
    rmdir_r,
};
use permitit::Permit;
use thiserror::Error;
use tracing::{
    debug,
    error,
    info,
    trace,
};

use super::{
    Package,
    source::SourceError,
};
use crate::{
    exec, package::{
        alias::gather_all_aliases, dep::DepKind, FormError
    }, utils::file::mtime, CONFIG
};

const MERGED: &str = "/var/lib/to/chroot/merged";

#[rustfmt::skip]
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Failed to clean overlay")]
    CleanOverlay,

    #[error("Failed to setup overlay")]
    SetupOverlay,

    #[error("Failed to fetch sources")]
    FetchSources(#[from] SourceError),

    #[error("Failed to populate overlay")]
    PopulateOverlay,

    #[error("Failed to execute pre-build hook")]
    PreBuildHook,

    #[error("Failed to build")]
    Build,

    #[error("Failed to resolve dependencies")]
    ResolveDeps(#[from] FormError),

    #[error("Failed to cache")]
    Cache,

    #[error("Failed to save distfile")]
    SaveDistfile,

    #[error("Shouldn't build")]
    ShouldntBuild,
}

impl Package {
    pub fn build(&self, force: bool) -> Result<(), BuildError> {
        // If we shouldn't build, and the build isn't forced, exit early
        if !self.should_build() && !force {
            return Err(BuildError::ShouldntBuild)
        }

        clean_overlay()?;
        setup_overlay()?;
        self.fetch_sources()?;
        self.populate_overlay()?;
        self.pre_build_hook()?;
        self.chroot_and_run()?;
        self.cache_stuff()?;
        self.save_distfile()?;

        Ok(())
    }

    fn should_build(&self) -> bool {
        let Some(pm) = mtime(self.pkgfile()) else { return true };
        let Some(dm) = mtime(self.distfile()) else { return true };
        pm > dm
    }

    fn pre_build_hook(&self) -> Result<(), BuildError> {
        debug!("Checking for pre-build steps for {self}...");
        let pkgfile = &self.pkgfile();

        exec!(
            "
            source {pkgfile}
            if is_function p; then
                echo 'Executing pre-build steps for {self}'
                p
            fi
            ",
            pkgfile = pkgfile.display(),
        )
        .map_err(|_| BuildError::PreBuildHook)
    }

    // NOTE: Dependencies should be installed after the chroot is entered
    fn populate_overlay(&self) -> Result<(), BuildError> {
        let name = &self.name;
        info!("Populating overlay for {name}");
        // TODO: Consider dropping `/etc/to/exclude` support
        // - Not sure if I wanna do this because I already wrote and used `il()` :shrug:
        for path in ["B", "D", "S", "etc/to"] {
            mkdir_p(Path::new(MERGED).join(path)).map_err(|_| BuildError::PopulateOverlay)?
        }

        exec!(
            r#"
            cd {MERGED}

            cp -vf {}                               pkg     # copy pkg file
            cp -vf /usr/share/to/scripts/runner.sh  runner  # copy runner

            if [ -d {}/A ]; then cp -af --no-preserve=xattr {}/A A; fi

            cp -vf /etc/resolv.conf                 etc/resolf.conf
            if [ -f /etc/to/config.toml ]; then cp -vf /etc/to/config.toml etc/to/config.toml; fi
            echo 'usr/share/doc'                >   etc/to/exclude
            echo 'usr/share/licenses'           >>  etc/to/exclude
        "#,
            self.pkgfile().display(),
            self.pkgdir().display(),
            self.pkgdir().display(),
        )
        .map_err(|_| BuildError::PopulateOverlay)?;

        if !self.dependencies.is_empty() {
            debug!("Copying dependencies to overlay")
        }

        fn copy_to_chroot(path: PathBuf) -> Result<(), BuildError> {
            let dest = Path::new(MERGED).join(
                path.strip_prefix("/")
                    .map_err(|_| BuildError::PopulateOverlay)?,
            );
            mkf_p(&dest).map_err(|_| BuildError::PopulateOverlay)?;
            copy(&path, dest).map(drop).map_err(|_| {
                error!("Failed to copy {} to chroot", path.display());
                BuildError::PopulateOverlay
            })
        }

        for source in &self.sources {
            // trace!("Copying over source {source:?}");
            let source_path = source.path(self);
            let source_dest = Path::new(MERGED).join("S").join(&source.dest);

            if source_path.is_dir() {
                dircpy::copy_dir(&source_path, &source_dest)
                    .map_err(|_| BuildError::PopulateOverlay)?;
            } else {
                copy(&source_path, &source_dest).map_err(|_| BuildError::PopulateOverlay)?;
            }
        }

        let deps = self.collect_chroot_deps()?;

        #[rustfmt::skip]
        for dep in &deps {
            let files = [dep.distfile(), dep.pkgfile(), dep.sfile()];
            for file in files {
                copy_to_chroot(file)
                    .map_err(|_| BuildError::PopulateOverlay)?;
            }

            debug_assert!(gather_all_aliases().len() > 10);

            // Replicate alias structure in chroot
            let alias_paths = dep.alias_pkgdirs();
            trace!("Aliases for {dep:-}: {alias_paths:#?}");

            #[cfg(debug_assertions)]
            if &dep.name == "ogg" {
                assert!(Package::from_s_file("ogg").unwrap().find_aliases().iter().any(|a| a.name == "libogg"));

                assert_eq!(Package::from_s_file("ogg").unwrap().find_aliases(), dep.find_aliases());
                assert!(dep.find_aliases().iter().any(|a| a.name == "libogg"));

                dbg!(&alias_paths);
                debug_assert!(alias_paths.contains(&PathBuf::from("/var/db/to/pkgs/libogg")));
            }

            for alias_path in alias_paths {
                let symlink = Path::new(MERGED).join(alias_path.strip_prefix("/").expect("Alias path should be absolute"));
                debug!("Symlinking '{}' -> '{}'", symlink.display(), &dep.name);

                fs::symlink(&dep.name, &symlink).map_err(|_| BuildError::PopulateOverlay)
                    .permit_if(alias_path.read_link().map(|p| p.to_string_lossy() == dep.name).unwrap_or(false))?;
                debug_assert_eq!(symlink, Path::new(MERGED).join("var/db/to/pkgs").join(alias_path.file_name().unwrap()));
                debug_assert_eq!(symlink.read_link().unwrap(), PathBuf::from(&dep.name));
            }

            // trace!("Copied over dependency {dep:-}")
        }

        // Write chroot/deps
        let deps_str = deps
            .iter()
            .map(|p| p.name.clone())
            .intersperse(" ".to_string())
            .collect::<String>();
        let deps_str = deps_str.trim();

        if deps_str.is_empty() {
            debug!("Not writing deps file since {self:-} has no dependencies");
        } else {
            let deps_file = format!("{MERGED}/deps");
            write(deps_file, deps_str).map_err(|_| BuildError::PopulateOverlay)?; // deps file
            debug!("Wrote deps file for {self}");
            trace!("Deps_str: {deps_str}");
        }

        Ok(())
    }

    fn chroot_and_run(&self) -> Result<(), BuildError> {
        info!("Entering chroot for {self}");
        exec!(
            r#"
        chroot {MERGED} \
            /usr/bin/env -i             \
                MAKEFLAGS="{makeflags}" \
                RUSTFLAGS="{rustflags}" \
                CXXFLAGS="{cflags}"     \
                FCFLAGS="{cflags}"      \
                CFLAGS="{cflags}"       \
                FFLAGS="{cflags}"       \
                TO_TEST={tests}         \
            /runner
        "#,
            makeflags = &CONFIG.makeflags,
            cflags = &CONFIG.cflags,
            rustflags = &CONFIG.rustflags,
            tests = CONFIG.tests,
        )
        .map_err(|_| BuildError::Build)
    }

    fn save_distfile(&self) -> Result<(), BuildError> {
        mkdir_p(self.distdir()).map_err(|_| BuildError::SaveDistfile)?;
        exec!(
            "cp -vf '/var/lib/to/chroot/upper/pkg.tar.zst' '{}'",
            self.distfile().display()
        )
        .map_err(|_| BuildError::SaveDistfile)?;

        info!("Saved distfile for {self}");
        Ok(())
    }

    /// # """"Cache"""" reusable stuff
    fn cache_stuff(&self) -> Result<(), BuildError> {
        const LOWER: &str = "/var/lib/to/chroot/lower";
        const UPPER: &str = "/var/lib/to/chroot/upper";

        // Cache make-ca certificates
        if self.dependencies.iter().any(|d| d.name == "make-ca") {
            debug!("Caching make-ca certificates if needed");

            mkdir_p(Path::new(LOWER).join("etc/ssl")).map_err(|_| BuildError::Cache)?;
            mkdir_p(Path::new(LOWER).join("etc/pki")).map_err(|_| BuildError::Cache)?;
            exec!(
                "
                if [ -d {UPPER}/etc/ssl/certs ]; then
                    cp -af --no-preserve=xattr {UPPER}/etc/ssl {LOWER}/etc/
                fi

                if [ -d {UPPER}/etc/pki ]; then
                    cp -af --no-preserve=xattr {UPPER}/etc/pki {LOWER}/etc/
                fi
                "
            )
            .map_err(|_| BuildError::Cache)?;

            exec!(
                "
                if [ -d {UPPER}/etc/pki ]; then
                    cp -af --no-preserve=xattr {UPPER}/etc/pki {LOWER}/etc
                fi
                "
            )
            .map_err(|_| BuildError::Cache)?;
        }

        // Cache rustup toolchains to avoid redownloading them
        if self.dependencies.iter().any(|d| d.name == "rust") {
            debug!("Caching rustup toolchains if needed");

            // TODO: Copying bs may happen here where nightly gets copied to nightly/nightly.
            // Testing needed. Probably use rsync if that happens.
            exec!(
                "
                if [ -d {UPPER}/opt/rustup/toolchains ]; then
                    cp -af --no-preserve=xattr {UPPER}/opt/rustup/toolchains {LOWER}/opt/rustup/
                fi

                if [ -d {UPPER}/opt/rustup/update-hashes ]; then
                    cp -af --no-preserve=xattr {UPPER}/opt/rustup/update-hashes {LOWER}/opt/rustup/
                fi
                "
            )
            .map_err(|_| BuildError::Cache)?;
        }

        Ok(())
    }
}

fn setup_overlay() -> Result<(), BuildError> {
    exec!(
        r#"
        cd        /var/lib/to/chroot

        # extract the stage3 if it's absent
        if [ ! -d lower/dev ]; then 
            tar xpf {stagefile} -C lower
        fi

        mount -vt overlay overlay -o lowerdir=lower,upperdir=upper,workdir=work merged
        mount -v --bind /dev merged/dev
        mount -vt devpts devpts -o gid=5,mode=0620 merged/dev/pts
        mount -vt proc proc merged/proc
        mount -vt sysfs sysfs merged/sys
        mount -vt tmpfs tmpfs merged/run
        "#,
        stagefile = CONFIG.stagefile,
    )
    .map_err(|_| BuildError::SetupOverlay)
}

fn clean_overlay() -> Result<(), BuildError> {
    let chroot = Path::new("/var/lib/to/chroot");
    mkdir_p(chroot).map_err(|_| BuildError::SetupOverlay)?;

    exec!(
        r#"
        if mountpoint -q {chroot}/merged; then
            umount -lR {chroot}/merged
        fi
        "#,
        chroot = chroot.display(),
    )
    .map_err(|_| BuildError::CleanOverlay)?;

    rmdir_r(chroot.join("upper")).map_err(|_| BuildError::CleanOverlay)?;
    rmdir_r(chroot.join("work")).map_err(|_| BuildError::CleanOverlay)?;

    for dir in ["lower", "merged", "upper", "work"] {
        mkdir_p(chroot.join(dir)).map_err(|_| BuildError::SetupOverlay)?;
    }

    Ok(())
}

/// # Returns the order in which all packages should be built
///
/// This is only used when building *every* package
pub fn get_build_order(mut all_packages: Vec<Package>) -> Vec<Package> {
    debug!("Resolving build order... (this may take a while)");
    let mut order: Vec<Package> = Vec::new();
    let original_len = all_packages.len();

    // We initiate a loop. In that loop, we loop through all the packages, pushing them to the
    // build order if all their dependencies are already in the order. The first iteration of the
    // lower loop will push packages with zero dependencies to the build order. The second will
    // push the packages which depend on the packages in the order from the first iteration, and so
    // on.
    //
    // The upper loop will break when the order length is equal to the length of all the packages,
    // meaning the order has been resolved.
    //
    // If the build order length equals the previous build order length, that is, if an iteration
    // and its subsequent iteration make no progress, we panic, since a circular dependency was
    // detected, or dependencies could not be resolved, or something along those lines.
    let mut previous_order_len;
    loop {
        previous_order_len = order.len();

        // Break once we've resolved everything.
        if previous_order_len == original_len {
            break;
        }

        trace!("Looping since order length ({}) is less than total length ({original_len})", order.len());
        let mut i = 0;
        while i < all_packages.len() {
            let order_names = order.iter().map(|p| p.name.as_str()).collect::<Vec<_>>();
            let pkg = &all_packages[i];

            // Find build and required dependencies, extracting their names. This is done to avoid
            // cycles when runtime dependencies are treated without nuance.
            let mut dependencies = pkg.dependencies.iter().filter(|d| matches!(d.kind, DepKind::Build | DepKind::Required))
                .map(|d| d.to_package().unwrap().name);

            if dependencies.all(|d| order_names.contains(&d.as_str())) {
                let pkg = all_packages.remove(i);
                trace!("Pushing package {pkg:-} to order since all its dependencies are in the build order");
                order.push(pkg);
            } else {
                trace!("Skipping package {:-} since the build order doesn't contain all its dependencies", all_packages[i]);
                i += 1;
            }
        }

        // Complain when no progress has been made -- typically means a circular dependency. This
        // shouldn't happen, but if it does, let's also provide some useful debugging information.
        if previous_order_len == order.len() {
            debug!("Build order: {:#?}", order.iter().map(|p| p.name.as_str()).collect::<Vec<_>>());
            for pkg in all_packages {
                debug!("Dependencies for {pkg:-}: {:#?}", pkg.dependencies.iter().map(|d| d.to_package().unwrap().name).collect::<Vec<_>>());
            }

            debug!("Build order length: {previous_order_len}");
            error!("Got stuck resolving build order");
            exit(1);
        }
    }

    debug_assert_eq!(order.len(), original_len); // sanity check
    debug!("Resolved build order");
    order
}

#[cfg(test)]
mod test {
    use crate::package::all_package_names;
    use crate::package::DepKind;
    use super::*;

    #[test]
    fn border() {
        let all_packages = all_package_names().iter().map(|p| Package::from_s_file(p).unwrap()).collect::<Vec<_>>();
        let total_len = all_packages.len();
        let order = get_build_order(all_packages);

        eprintln!("{order:#?}");

        // assert that samu would be built before ninja since the former has fewer dependencies
        assert!({
            order.iter().position(|p| p.name == "samu") < order.iter().position(|p| p.name == "ninja")
        });

        // assert that efibootmgr, which depends on efivar, would be built after efivar
        assert!({
            order.iter().position(|p| p.name == "efivar") < order.iter().position(|p| p.name == "efibootmgr")
        });

        // assert that glibc would be built before gcc
        assert!({
            order.iter().position(|p| p.name == "glibc") < order.iter().position(|p| p.name == "gcc")
        });

        // assert polkit is built after elogind
        assert!({
            order.iter().position(|p| p.name == "elogind") < order.iter().position(|p| p.name == "polkit")
        });

        // assert the length is correct
        assert_eq!(order.len(), total_len);
    }

    #[test]
    fn depression() {
        // The xorg-server package is chosen since it has all sorts of dependencies
        let deps = Package::from_s_file("xorg-server")
            .unwrap()
            .resolve_deps(|_| true);

        // Ensure dependency information is maintained
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Required));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Runtime));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Build));

        // Ensure all dependency packages have `depkind` and confirm `is_dependency()` works
        assert!(deps.iter().all(|d| d.is_dependency()));

        // Print stuff out
        for dep in deps {
            eprintln!("{:+}", dep.to_dep().unwrap())
        }
    }
}
