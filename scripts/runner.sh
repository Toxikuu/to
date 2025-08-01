#!/usr/bin/env bash
set -euo pipefail
# Runner script for `to`
# This gets executed in the chroot


# Hold the user's hand
# TODO: Explain at a high level how the overlay file system works, and how `to` takes advantage of it, and how any files always wanted in the build chroot should be installed to lower
# TODO: Explain that `umount -vR /var/lib/to/chroot/merged && rm -rf /var/lib/to/chroot` may be executed to start fresh from the stagefile
# TODO: Cover those in mdbook documentation probably
if [ ! -e /usr/share/to/envs/base.env ]; then
    cat << 'EOF' >&2

    ERROR: Missing base environment
    You most likely haven't installed `to` to the build chroot
    You may do so by navigating to the `to` source directory and executing the following command:

    sudo make DESTDIR=/var/lib/to/chroot/lower install

    Rerun that command whenever `to` is updated
    There will probably be a better way to do this in the future

EOF
    exit 9
fi


# Source base environment and enter build directory
source /usr/share/to/envs/base.env
cd "$B"


# Install dependencies, if any
if [ -f /deps ]; then
    echo "Installing dependencies..."
    # shellcheck disable=SC2046
    to install -ds $(</deps)
fi

# Source package
tource /pkg


# Extract zips and tarballs; copy other sources to $B
register_source() {
    echo "Registering source file '$src'"

    if [[ "$src" == *".t"*"z"* ]]; then
        # tarballs
        tar xf "$src" -C "$B" || die "Failed to register $src (tar)"
    elif file "$src" | grep -q 'Zip archive data'; then
        # zips
        unzip "$src"  -d "$B" || die "Failed to register $src (zip)"
    else
        # git repos, patches, or random shit
        cp -af --no-preserve=xattr "$src" "$B" || die "Failed to register $src (copy)"
    fi
}


# Follow custom extraction logic if defined, otherwise extract all sources to $B
if is_function xt; then
    xt
else
    shopt -s nullglob
    for src in "$S/"*; do
        register_source
    done
fi


# If there's only one directory in $B, enter it; else, let `b` handle it
readarray -t dirs < <(find . -mindepth 1 -maxdepth 1 -type d)
if [ "${#dirs[@]}" -eq 1 ]; then
    cd "$dirs"
    echo "Entered source directory '$dirs' (only)"
fi


# Try to infer environments from dependencies if unspecified
if ! type b 2>/dev/null | grep -q 'with'; then
    if [[ ${d[*]} =~ "b,rust" ]]; then with rust; fi
    if [[ ${d[*]} =~ "b,go" ]]; then with go; fi

    if [[ ${d[*]} =~ "b,meson" ]] || [[ "${d[*]}" =~ "b,muon" ]]; then
        with meson
    elif [[ ${d[*]} =~ "b,cmake" ]]; then
        with cmake
    fi
fi


# Execute build
if is_function b; then
    echo "Executing build instructions"
    b
else
    echo "No build instructions to execute" >&2
fi


# Run opts
echo "Running opts"
/usr/share/to/scripts/opts/run


# Run QA checks
echo "Running QA checks"
/usr/share/to/scripts/qa/run


# Execute tests if enabled
if is_function t && $TO_TEST; then
    echo "Executing test instructions"
    t
fi


# Check if anything is in $D
is_dest_populated() {
    [ -n "$(find "$D" -mindepth 1 -maxdepth 1)" ]
}


# Create a package
if is_dest_populated; then

    # Record package manifest and create tarball
    echo "Creating distfile"
    find "$D" -mindepth 1 |
        sed -e "s,^$D/,," -e '/^MANIFEST/d' > "$D/MANIFEST"

    cd "$D"
    tar cf - -- * | zstd -f -T0 -19 -o "/pkg.tar.zst" &>/dev/null

else

    echo "Dest is not populated" >&2

fi
