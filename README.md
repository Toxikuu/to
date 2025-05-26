# To

<!-- TODO: update the below line when I'm comfortable with the state of matter of
this project -->
**Stupid-simple, rock-solid (EVENTUALLY CURRENTLY WIP AND NOT SOLID AT ALL DO
NOT USE HERE BE DRAGONS) package manager for LFS**

## Features
- Containerized builds via OverlayFS and stage files
- Surprisingly half-decent dependency resolution
- Distribution tarballs (hereafter referred to as distfiles) (featuring zstd!)!!
- Utilities to hold your hand while you maintain packages (aka mostly foolproof
bash scripts, emphasis on the mostly)
- Manifest system (meaning stray files get deleted and shit)

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
- LFS[^2]
- LFStage (currently required, but eventually optional -- it builds the chroot
stage files)

<!-- TODO: Verify whether LFS is required cus lowkey idt it is -->
[^2]: If you wanna try it somewhere else have fun, but this expects LFS.

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
WORKING ON IT. SOON I PROMISE.

## Documentation
NOT WORKING ON IT YET. PROBABLY NOT SOON. NO PROMISES.

## TODO
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
- [ ] Add `to data`
    - [ ] Should display data about `to`, including the number of installed
    packages, the total number of packages, the health (with a flag), the number
    of outdated packages, the number of commit-versioned packages
- [ ] Improve `to view`
    - [x] Add `--dependencies` and `--deep-dependencies`
    - [ ] Add `--dependants`
    - [ ] Add `--distfiles` to show available distfiles for a package
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
- [ ] Write an explanation of how `to` works
    - [ ] Also write documentation (eventually)
- [ ] Fucking finish this readme
    - [ ] Write an information section talking about how `to` has optional
    features for maintainers, servers, and end users (so you can have a single
    machine that builds packages and runs a server and just download and install
    all the packages to your other machines)
    - [ ] Also maybe format things properly and make this look like it wasnt
    written in 5 minutes while high off sleep deprivation
- [x] Write `to lint`
    - [x] Add a lint "IlOpportunity" for missed il usage, similar to def's lint
    - [ ] Remove the aliases lint since it appears to neither work nor matter
        - [ ] Update the alias thing does matter, but only for deep
        dependencies. Not sure how I plan to work around this ngl.
- [x] Add post-build QA checks
     - [ ] Make them less shit
- [x] Add `--debug` for `to view`, and change the default behavior to give
      something human readable.
- [x] Add `to alias`
    - [x] Drop support for implicit package generation
- [x] Add message support
- [ ] Fork my reqs.sh from LFStage and adapt it for use here
- [x] Add `to sync`
- [ ] Add `to search`
- [ ] Add `to --version`
- [x] Cache the output of `to vf`. This cache should reset every 4 hours, but
  should be overrideable by a flag.
- [ ] Add to-specific data in /var/db/to/data/_/
    - [ ] Have a file containing the number of installed packages
    - [ ] Have a file logging the current action (eg. building tree, installing
    popt, removing glibc)
    - [ ] Have a file logging the latest package actions (installs, updates,
    removals, etc.)
- [x] Drop c_c in base.env because it's very flaky
    - [ ] Fix associated build failures
<!-- - [ ] Record configure options to /var/db/to/data/$n/cfg. This would be done by -->
<!--   packaging /_cfg into the tarball, similar to MANIFEST. -->
- [ ] Provide an official stage file. Ensure the presence of `zstd`. Also
  automatically install `to` to it. This stage file should be provided as a
  release asset.
