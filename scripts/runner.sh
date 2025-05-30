#!/usr/bin/env bash
set -euo pipefail
# Runner script for `to`
# This gets executed in the chroot


# Source stuff
source /usr/share/to/envs/base.env
source /pkg


# Enter the build directory
cd "$B"


# Install dependencies, if any
if [ -f /deps ]; then
    echo "Installing dependencies..."
    # shellcheck disable=SC2046
    to install --suppress-messages $(</deps)
fi


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
        cp -af "$src"    "$B" || die "Failed to register $src (copy)"
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
else
    die "Multiple source directories. Specify which in \`b()\`."
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


# Run QA checks
echo "Running QA checks"
/usr/share/to/scripts/qa/qa.sh


# Check if anything is in $D
is_dest_populated() {
    [ -n "$(find "$D" -mindepth 1 -maxdepth 1)" ]
}


# Create a package
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
