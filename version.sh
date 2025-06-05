#!/usr/bin/env bash
# A cute little script to generate the version

get_commit() {
    if ! command -v git &>/dev/null; then
        echo "unknown"
    else
        git rev-parse --short=8 HEAD
    fi
}

get_dev() {
    if ${DEV:-true}; then
        echo "-dev"
    fi
}

CRATE="$(grep '^version' Cargo.toml | cut -d'"' -f2)"
COMMIT="$(get_commit)"
DEV="$(get_dev)"

echo "$CRATE-$COMMIT$DEV"
