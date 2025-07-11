# To

<!-- TODO: update the below line when I'm comfortable with the state of this
           project -->
**Relatively simple, rock-solid (EVENTUALLY CURRENTLY WIP AND NOT SOLID AT ALL DO
NOT USE HERE BE DRAGONS) package manager for LFS**

## Features
- Containerized builds via OverlayFS and stage files
- Surprisingly half-decent dependency resolution
- Distribution tarballs (hereafter referred to as distfiles) (featuring zstd!)!!
- Utilities to hold your hand while you maintain packages (aka mostly foolproof
bash scripts, emphasis on the mostly)
- Manifest system (meaning stray files get deleted and shit)
- (Fast) upstream version checking

## Non-Features
So remember how I said `to` is stupid simple? I explicitly omit basic features,
such as:
<!-- TODO: update the below bullet point when `to` is not in its current state -->
<!-- TODO: write documentation on repo maintenance and update the below bullet
           point -->
- Repositories (there is only one. you must maintain your own, keeping with the
spirit of LFS and all. you can fork
[mine](https://github.com/Toxikuu/to-pkgs.git) as reference. but the
documentation won't come until much later)
- Backups (you're responsible for making your own. please make your own backups
if you're stupid or brave enough to use this, especially in its current state)
- Split packages (that shit looks gross in a build file and it's complex)

## Dependencies
### Build
- Rust
### Runtime
- OverlayFS support in your kernel (CONFIG_OVERLAY_FS=*)
- Curl (used to download shit)
- Git (used to git shit)
- LFS[^1]
- [LFStage](https://github.com/Toxikuu/lfstage.git) with
[to-lfstage](https://github.com/Toxikuu/to-lfstage.git) (currently required, but
eventually optional -- it builds the chroot stage files)

<!-- TODO: Verify whether LFS is required cus lowkey idt it is -->
[^1]: If you wanna try it somewhere else have fun, but this expects LFS.

## Installation
Please don't. I made this for myself and myself alone. This will hopefully be
stable and documented enough for other people to use eventually, but currently
this is just for me. You can use it if you want, but good luck figuring out what
the fuck all my one character variables mean.

If you hate yourself and disregard warnings of dragons:
```bash
./configure --prefix=/usr # You are running LFS after all
make
make install
```

## Pro-Tips
~WORKING ON IT. SOON I PROMISE.~ ok so i lied with this promise its gonna be a
bit because there is so much shit to do.

## Documentation
NOT WORKING ON IT YET. PROBABLY NOT SOON. NO PROMISES.

## TODO
- [ ] Add a flag `--no-dependencies` to skip resolving and installing
  dependencies for `to install`
- [ ] Add an option to the config to disable logging to stdout
    - Also add a framework for changing config options for a single run via a
      flag
- [x] Add support for `--root=/path/to/destdir` to the install subcommand
    - Don't bother trying to get `i()` to work with this, and simply mention
      that post-install instructions aren't supported for `--root` installs
- [x] Allow `mk check || true` and similar commands
    - [ ] Test this
- [ ] Release the stage file somewhere and download it when installing
    - [ ] Add a variable to the makefile to avoid downloading the default
    stagefile
- [ ] Add a config option to enable all logs that are currently commented out
    - Maybe a `really_fucking_loud_logs` bool in the config
- [x] Instead of sprinkling `laway` throughout my build scripts, include it in a
post-build steps system that runs before QA. This system should be similar to
QA. Probably call it opts. Move strip into opts. Should allow for stuff like
opts=(!strip !la)
- [x] Add --force for `to build`. Make the build not wanna build by default,
only building if the pkgfile is newer than the distfile.
- [x] Make cli modular
- [ ] Fix message formatting as visible in `tzdata`
- [x] Don't install runtime dependencies to the build environment.
    - [x] There needs to be some way to check whether a package is being
    installed in the build environment. Use the existence of /D.
- [x] Set the LAST_MODIFIED header with `fmt_http_date()` from `httpdate`
- [ ] Track package install size, probably using `sighs` and `size`
    - [ ] Distfile size should be trivial, but installed size will require some
    work. It should probably be written as metadata to the sfile.
- [x] Make `to edit` not rely on the stale s file
    - Fixed by running `to lint` after `to generate`
- [ ] Drop the 2 pkg-add template once I've transferred all the packages I want
- [x] Stop using /var/cache for everything. lol.
- [x] Add zstd to LFStage
- [x] Use the pardl poc instead of curl for pull
- [x] Add `to data`
    - [ ] If called without the package positional argument, should display data
    about `to`, including the number of installed packages, the total number of
    packages, the health (with a flag), the number of outdated packages, the
    number of commit-versioned packages.
- [ ] Improve `to view`
    - [x] Add `--dependencies` and `--deep-dependencies`
    - [x] Add `--dependants`
    - [x] Add `--tree` to view the file tree of a package's distfile
    - [ ] Add `--distfiles` to show available distfiles for a package
    - [ ] Add `--upstream` to show a package's upstream
    - [ ] Add `--source` to show a package's source
    - [ ] Add `--outdated` to show only outdated packages
    - [ ] Add `--installed` to show only installed (and outdated) packages
    - [ ] Add `--available` to show only available packages
    - [ ] Add `--pkg` to cat the pkgfile
    - [ ] Add `--manifest` to display the manifest
        - [ ] Displaying the manifest will check the manifest in the distfile,
        since that one is complete and contains no exclusions, but fall back to
        the installed manifest if the distfile's manifest doesn't exist or
        couldn't be accessed. Consider dropping exclusions for simplicity.
        - [ ] If I drop exclusions, it should prefer the system manifest, and
        fall back to the tarball one, since the system would be faster.
    - [x] Adjust the format for outdated packages to be 'name@iv -> v'
- [ ] Also write documentation (eventually)
    - [ ] Man pages
    - [ ] mdbook docs
        - [ ] Should dive into how `to` works and rationale behind design
- [ ] Fucking finish this readme
    - [ ] Write an information section talking about how `to` has optional
    features for maintainers, servers, and end users (so you can have a single
    machine that builds packages and runs a server and just download and install
    all the packages to your other machines)
    - [ ] Also maybe format things properly and make this look like it wasnt
    written in 5 minutes while high off sleep deprivation
- [x] Write `to lint`
    - [x] Add a lint "IlOpportunity" for missed il usage, similar to def's lint
    - [x] Remove the aliases lint since it appears to neither work nor matter
        - [x] Update the alias thing does matter, but only for deep
        dependencies. Not sure how I plan to work around this ngl. Aliases are
        now supported in deep dependencies.
- [x] Add post-build QA checks
     - [x] Make them less shit
        - [x] Make them modular
        - [x] Add a check for static libraries, and libtool archives
        - [ ] Add a check for missing pc files, bin, lib, etc.
        - [x] Add e.g. 'qa=(!static)' support to the pkg parser
            - [x] Allow qa checks to be toggled on or off
- [x] Add `--debug` for `to view`, and change the default behavior to give
      something human readable.
- [x] Add `to alias`
    - [x] Drop support for implicit package generation
- [x] Add message support
- [ ] Fork my reqs.sh from LFStage and adapt it for use here
- [x] Add `to sync`
- [ ] Add `to search`
- [x] Add `to --version`
- [x] Cache the output of `to vf`. This cache should reset every 4 hours, but
  should be overrideable by a flag.
    - [ ] Add an option to use the cache, even if it's stale
- [ ] Add to-specific data in /var/db/to/data/_/
    - [ ] Have a file containing the number of installed packages (not sure
    about this one)
    - [ ] Have a file logging the current action (eg. building tree, installing
    popt, removing glibc)
    - [ ] Have a file logging the latest package actions (installs, updates,
    removals, etc.)
- [x] Drop c_c in base.env because it's very flaky
    - [x] Fix associated build failures
- [ ] Stop writing configure options to /_cfg_opts
    - Tbh keep it for now its nice for debugging whether the environments
    picked up the options.
- [ ] Provide an official stage file. Ensure the presence of `zstd`.
    - [x] Make an LFStage profile for it
    - [ ] Once `to` is relatively stable, install it to the stage file profile
    (not so sure about this one either; i kinda like destdiring it)
- [ ] Only check health upon request; make the output more useful
- [x] Support rebuilding the entire repo
    - This would be done by resolving the order in which all packages should be
      built and building.

### IDEAS
- [ ] Use bubblewrap instead of chroot, allowing for unprivileged building
    - Maybe pair with fakeroot?
