#!/usr/bin/env bash
set -euo pipefail

usage='Usage: tools/package.sh REVISION NEW_OUTPUT_DIRECTORY'
if [[ ${1:-} == --help ]]; then
    printf '%s\n' "$usage"
    printf 'Build the compositor from a clean export of REVISION and write a release tarball and SHA256SUMS.\n'
    exit 0
fi
if (( $# != 2 )); then
    printf '%s\n' "$usage" >&2
    exit 2
fi
revision=$(git rev-parse --verify "$1^{commit}")
output=$2
mkdir -- "$output"
output=$(realpath "$output")
work=$(mktemp -d /tmp/compust-package.XXXXXX)
trap 'rm -rf "$work"' EXIT

mkdir "$work/source" "$work/package"
git archive "$revision" | tar -x -C "$work/source"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$work/source/Cargo.toml" | head -n 1)
name="compust-$version-$(uname -m)-linux"
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct "$revision")
export SOURCE_DATE_EPOCH
# Keep local paths out of panic locations so the binary does not depend on the build machine.
cargo_home=${CARGO_HOME:-$HOME/.cargo}
export RUSTFLAGS="--remap-path-prefix=$cargo_home=/cargo --remap-path-prefix=$work=/build"
# rustup selects the pinned toolchain from the exported source directory.
(cd "$work/source" && cargo build --release --locked --bin compust --target-dir "$work/target")

directory="$work/package/$name"
mkdir "$directory"
install -m 0755 "$work/target/release/compust" "$directory/compust"
for file in LICENSE README.md README.pt-BR.md compust.example.toml; do
    install -m 0644 "$work/source/$file" "$directory/$file"
done
{
    printf 'version %s\nrevision %s\nsource_date_epoch %s\n' "$version" "$revision" "$SOURCE_DATE_EPOCH"
    printf 'command cargo build --release --locked --bin compust\n'
    (cd "$work/source" && rustc -vV)
} >"$directory/BUILDINFO"
chmod 0644 "$directory/BUILDINFO"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$SOURCE_DATE_EPOCH" \
    --format=gnu -C "$work/package" -cf - "$name" | gzip -n -9 >"$output/$name.tar.gz"
(cd "$output" && sha256sum "$name.tar.gz" >SHA256SUMS)
printf 'Packaged %s\n' "$output/$name.tar.gz"
