#!/usr/bin/env bash
# Build slurmer's source RPM into <outdir>.
#
# Packs the committed tree at HEAD (uncommitted changes are left out),
# vendors every crate so the RPM build needs no network, lists the vendored
# crates' licenses, and sets the spec's Version from Cargo.toml.
#
# COPR runs this through .copr/Makefile. Locally: make-srpm.sh /tmp/srpm
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <outdir>" >&2
    exit 2
fi

top=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$1"
outdir=$(cd "$1" && pwd)
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$top/Cargo.toml" | head -n 1)
name=slurmer

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/SOURCES" "$work/SPECS" "$work/vendor-root"

# COPR clones the repo as another user than the one running this script.
git -C "$top" -c safe.directory='*' archive --format=tar.gz \
    --prefix="$name-$version/" -o "$work/SOURCES/$name-$version.tar.gz" HEAD

(
    cd "$top"
    cargo vendor --quiet --locked --versioned-dirs "$work/vendor-root/vendor" > /dev/null
    cargo tree --quiet --locked --target all -e normal --prefix none --format '{p}: {l}' \
        | grep -v "^$name " | sort -u > "$work/vendor-root/LICENSE.dependencies"
)
XZ_OPT=-T0 tar -C "$work/vendor-root" -cJf "$work/SOURCES/$name-$version-vendor.tar.xz" \
    vendor LICENSE.dependencies

sed "s/^Version:.*/Version:        $version/" "$top/packaging/fedora/$name.spec" \
    > "$work/SPECS/$name.spec"

rpmbuild -bs \
    --define "_topdir $work" \
    --define "_srcrpmdir $outdir" \
    "$work/SPECS/$name.spec"
