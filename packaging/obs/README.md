# Open Build Service package

The RPM recipe packages Git snapshot
`681eae243cbf31d723928ce39317acd92a26243a`, after `0.3.0-beta.1`, as
`0.3.0~beta.1+git20261004.681eae2`. The binary's `--version` still reports
the upstream Cargo version; the RPM version and its installed `BUILDINFO`
identify this snapshot. This is not a new upstream release or Git tag.

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

## Project settings on OBS

The recipe depends on two settings in the project's meta configuration:

- Each Fedora repository lists the `update` repository before `standard`.
  The release repositories alone carry Rust 1.90 (Fedora 43) and 1.94
  (Fedora 44), and the package would stay unresolvable.
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
```

## Prepare sources

Run from the repository root, with Cargo and the pinned upstream toolchain
available. Choose a new output directory. `cargo vendor --locked` may fetch
the dependencies identified by `Cargo.lock`; the resulting RPM build needs
no network access.

```bash
set -euo pipefail
revision=681eae243cbf31d723928ce39317acd92a26243a
short=681eae2
output="$PWD/artifacts/obs-upload-$short"
epoch=$(git log -1 --format=%ct "$revision")
mkdir "$output"
mkdir "$output/work"
git archive --format=tar --prefix="compust-$short/" "$revision" |
    gzip -n -9 >"$output/compust-$short.tar.gz"
tar -xzf "$output/compust-$short.tar.gz" -C "$output/work"
(
    cd "$output/work/compust-$short"
    cargo vendor --locked --versioned-dirs vendor
)
mkdir "$output/work/compust-$short/.cargo"
cp packaging/obs/vendor-config.toml "$output/work/compust-$short/.cargo/config.toml"
tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" \
    --format=gnu -C "$output/work/compust-$short" -cf - .cargo vendor |
    zstd -T1 -19 -o "$output/compust-vendor-$short.tar.zst"
cp packaging/obs/compust.spec packaging/obs/compust.changes "$output/"
(
    cd "$output"
    sha256sum compust.spec compust.changes compust-*.tar.* >SHA256SUMS
)
```

Upload `compust.spec`, `compust.changes`, the source archive, the vendor
archive and `SHA256SUMS` into the `compust` package of `home:hashdefault`.
The archives are generated artifacts and must not be committed to Git.
An existing checkout's `target/`, local configuration and working-tree
changes are excluded by `git archive`.

For a later snapshot, update the full and short revisions, the date and
RPM version in the recipe, the `.changes` entry and these commands together.
Regenerate both archives, keeping the upstream lock file unchanged, and
verify the target build before describing it as published.

References: [openSUSE cargo-packaging](https://github.com/openSUSE-Rust/cargo-packaging)
and [OBS Rust vendoring](https://github.com/openSUSE-Rust/obs-service-cargo).
