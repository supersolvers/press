#!/usr/bin/env bash
set -euo pipefail

output=$(realpath -m "${1:-dist}")
version=$(sed -n 's/^version *= *"\([^"]*\)"/\1/p' Cargo.toml)
deb_version=$(dpkg-parsechangelog --show-field Version)
arch=$(dpkg --print-architecture)
if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ || ${deb_version%-*} != "$version" || $arch != amd64 ]]; then
    printf 'Unexpected package version or architecture (%s, %s, %s).\n' "$version" "$deb_version" "$arch" >&2
    exit 1
fi
if [[ ${GITHUB_REF_TYPE:-} == tag && ${GITHUB_REF_NAME:-} != "v$version" ]]; then
    printf 'Expected release tag v%s, found %s.\n' "$version" "${GITHUB_REF_NAME:-}" >&2
    exit 1
fi

# Rust comes from the pinned toolchain instead of the system packages.
command -v cargo rustc >/dev/null
dpkg-checkbuilddeps -d 'debhelper-compat (= 13), fontconfig, libxkbcommon-dev, pkgconf'

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/source"
git archive HEAD | tar -x -C "$work/source"
(
    cd "$work/source"
    dpkg-buildpackage -b -us -uc -d
)

mkdir -p "$output"
runtime="libpress1_${deb_version}_${arch}.deb"
development="libpress-dev_${deb_version}_${arch}.deb"
cp "$work/$runtime" "$work/$development" "$output/"
(cd "$output" && sha256sum "$runtime" "$development" > SHA256SUMS)
