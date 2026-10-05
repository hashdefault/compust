# Open Build Service package

The RPM recipe and Debian source package build snapshot
`0.3.0~beta.2+git20261005.3dfd920`, based on the 0.3.0-beta.2 prerelease,
from commit `3dfd920d3cffc0236d85331f9728ce6823231f8d`. Its version sorts
after beta.2 and before a final 0.3.0. The installed `BUILDINFO` names the
revision.

Both use the same two archives, named as Debian orig archives: the source
exported by `git archive`, and `orig-deps`, which holds the vendored crates
and the Cargo configuration that selects them. `dpkg-source` reads gzip, xz,
bzip2 and lzma but not zstd, so the dependency archive is xz.

The recipe targets openSUSE Tumbleweed and Fedora, x86_64, and was verified
on Tumbleweed and Fedora 44. It requires Rust and Cargo 1.95 or newer. It
uses openSUSE's `cargo-packaging` macros or, on Fedora, `cargo-rpm-macros`,
both with `--locked`; either set builds offline from the vendor archive.
The `%check` section runs the workspace tests, Xvfb pixel checks, Mesa
software GPU checks and validation of the installed example configuration.
No autostart entry or active desktop configuration is installed.

Before the parallel tests, the recipe creates `/tmp/.X11-unix` if absent.
On a fresh Tumbleweed container, simultaneous Xvfb directory creation left
some servers with only abstract sockets, while the test proxies require
filesystem sockets. Creating the directory once avoids that startup race
without skipping tests or changing their assertions.

The license directory holds the project license and every notice found in
the vendored dependencies. `%fdupes` hard-links identical copies, so each
text is stored once.

## Debian package

`debian/` is a `3.0 (quilt)` source package. It was built on Debian 13 with
Rust and Cargo 1.95 from `trixie-backports`; Debian 13 itself has 1.85, and
the Ubuntu releases on OBS stop at 1.91 (24.04) and 1.93 (26.04). The
resulting `.deb` depends only on `libc6 (>= 2.34)` and `libgcc-s1`, and was
installed and run on Debian 13, Ubuntu 22.04, Ubuntu 24.04 and the Linux
Mint 22 container image. One Debian 13 repository therefore serves all of
them.

`dpkg-source` unpacks the additional `orig-deps` component into `deps/`.
`debian/rules` writes a Cargo configuration that points at `deps/vendor`,
builds offline and locked, and runs the same tests and checks as the RPM
recipe. It keeps `dh_clean` out of `deps/vendor`: Cargo verifies every
vendored file, and `dh_clean` would delete the `Cargo.toml.orig` files. The
dependency notices go to `/usr/share/doc/compust/dependency-licenses`.

## Project settings on OBS

The packages depend on these settings in the project's meta configuration:

- Each Fedora repository lists the `update` repository before `standard`.
  The release repositories alone carry Rust 1.90 (Fedora 43) and 1.94
  (Fedora 44), and the package would stay unresolvable.
- The Debian 13 repository lists `backports` before `standard`, for the
  same reason.
- The `debuginfo` flag is enabled. OBS then moves the debug symbols into
  `compust-debuginfo` and `compust-debugsource` and strips the installed
  binary. With the flag off, the binary keeps its symbol table and rpmlint
  reports `unstripped-binary-or-object`.

```xml
<debuginfo>
  <enable/>
</debuginfo>
<repository name="Fedora_44">
  <path project="Fedora:44" repository="update"/>
  <path project="Fedora:44" repository="standard"/>
  <arch>x86_64</arch>
</repository>
<repository name="Debian_13">
  <path project="Debian:13" repository="backports"/>
  <path project="Debian:13" repository="standard"/>
  <arch>x86_64</arch>
</repository>
```

## Prepare sources

Run from the repository root, with Cargo and the pinned upstream toolchain
available. Choose a new output directory. `cargo vendor --locked` may fetch
the dependencies identified by `Cargo.lock`; the package builds need no
network access. `dpkg-source` and `dpkg-parsechangelog` come from `dpkg-dev`
on Debian and from the `dpkg` package on Arch.

```bash
set -euo pipefail
revision=3dfd920d3cffc0236d85331f9728ce6823231f8d
short=3dfd920
version='0.3.0~beta.2+git20261005.3dfd920'
output="$PWD/artifacts/obs-upload-$short"
epoch=$(git log -1 --format=%ct "$revision")
mkdir "$output"
mkdir "$output/work"
git archive --format=tar --prefix="compust-$short/" "$revision" |
    gzip -n -9 >"$output/compust_$version.orig.tar.gz"
tar -xzf "$output/compust_$version.orig.tar.gz" -C "$output/work"
(
    cd "$output/work/compust-$short"
    cargo vendor --locked --versioned-dirs vendor
)
mkdir "$output/work/compust-$short/.cargo"
cp packaging/obs/vendor-config.toml "$output/work/compust-$short/.cargo/config.toml"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" \
    --format=gnu -C "$output/work/compust-$short" -cf - .cargo vendor |
    xz -9 -T1 >"$output/compust_$version.orig-deps.tar.xz"

# Debian source package: the two archives plus packaging/obs/debian.
mkdir "$output/compust-$version" "$output/compust-$version/deps"
tar -xzf "$output/compust_$version.orig.tar.gz" --strip-components=1 \
    -C "$output/compust-$version"
tar -xJf "$output/compust_$version.orig-deps.tar.xz" -C "$output/compust-$version/deps"
cp -R packaging/obs/debian "$output/compust-$version/debian"
(
    cd "$output"
    SOURCE_DATE_EPOCH=$(dpkg-parsechangelog -l "compust-$version/debian/changelog" -S Timestamp)
    export SOURCE_DATE_EPOCH
    dpkg-source -b "compust-$version"
)
rm -rf "$output/compust-$version" "$output/work"

cp packaging/obs/compust.spec packaging/obs/compust.changes "$output/"
(
    cd "$output"
    sha256sum compust.spec compust.changes compust_* >SHA256SUMS
)
```

Upload every file in the output directory into the `compust` package of
`home:hashdefault`: the recipe, the `.changes` file, the two archives, the
`.dsc`, the `debian.tar.xz` and `SHA256SUMS`. Then remove the four
`compust_*` files of the older version from the package, its `.dsc` first:
OBS builds from the `.dsc` it finds, and the old archives would stay unused.
The archives are generated artifacts and must not be committed to Git. An
existing checkout's `target/`, local configuration and working-tree changes
are excluded by `git archive`.

For a later snapshot, update the full and short revisions, the date and
version in the recipe, in `debian/changelog` and `debian/rules`, the
`.changes` entry and these commands together. Give the version a later
date than the packaged one: with the same date, the two hashes alone would
decide which version is newer. Regenerate both archives, keeping the
upstream lock file unchanged, and verify each target build before
describing it as published.

References: [openSUSE cargo-packaging](https://github.com/openSUSE-Rust/cargo-packaging)
and [OBS Rust vendoring](https://github.com/openSUSE-Rust/obs-service-cargo).
