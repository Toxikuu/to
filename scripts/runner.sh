#!/usr/bin/env bash
# Runner script for `to`
# This gets executed in the chroot


# Enable strict mode :100:
set -eu -o pipefail


# Source stuff
source /usr/share/to/envs/base.env
source /pkg


# Enter the build directory
cd "${B:?}"


# Install dependencies, if any
if [ -f /deps ]; then
    echo "Installing dependencies..."
    # shellcheck disable=SC2046
    to install --suppress-messages $(</deps)
fi


# Extract zips and tarballs; copy other sources to $B
register_source() {
    echo "Registering source file '$src'"

    if file "$src" | grep 'Zip archive data'; then
        # zips
        unzip "$src"  -d "$B" || die "Failed to extract $src (zip)"
    elif [[ "$src" == *".t"*"z"* ]]; then
        # tarballs
        tar xf "$src" -C "$B" || die "Failed to extract $src (tar)"
    else
        # git repos, patches, or random shit
        cp -af "$src"    "$B" || die "Failed to extract $src (copy)"
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
    cd "$dirs" || die "Multiple source directories. Specify which in \`b()\`."
    echo "Entered source directory '$dirs' (only)"
fi


# Execute build
if is_function b; then
    echo "Executing build instructions"
    b
else
    echo "No build instructions to execute" >&2
fi


# Execute tests if enabled
if is_function t && $TO_TEST; then
    echo "Executing test instructions"
    t
fi


# Check if anything is in $D
is_dest_populated() {
    [ -n "$(find "$D" -mindepth 1 -maxdepth 1)" ]
}


# Create a package (execute the package function)
if is_dest_populated; then

    # Strip if not disabled
    if $TO_STRIP; then
        echo "Stripping binaries"
        find "${D}" -type f -executable -exec file {} + |
            grep 'not stripped' |
            cut -d: -f1         |
            xargs -r -- strip --strip-unneeded || true
    fi

    # Record package manifest and create tarball
    echo "Creating distfile"
    find "$D" -mindepth 1 |
        sed -e "s,^$D/,," -e '/^MANIFEST/d' > "$D/MANIFEST"

    cd "$D"
    tar cf - -- * | zstd -f -T0 -19 -o "/pkg.tar.zst" &>/dev/null

else

    echo "Dest is not populated" >&2

fi
