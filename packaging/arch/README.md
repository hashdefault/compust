# Arch Linux package

`PKGBUILD` builds the 0.3.0-beta.4 prerelease, tag `v0.3.0-beta.4`, from
GitHub's archive of the tag, in the form the Arch User Repository expects.
Cargo fetches the locked dependencies in `prepare()`; `build()` and `check()`
then run offline. `check()` runs the workspace tests, the Xvfb pixel checks,
Mesa's software GPU checks, and the validation of the example
configuration, as the Open Build Service recipes do. The package installs
`/usr/bin/compust`, the license, both READMEs, and `compust.example.toml`:
no autostart entry and no active configuration.

Build and install it on Arch Linux or a derivative that uses `pacman`:

```sh
git clone https://github.com/hashdefault/compust.git
cd compust/packaging/arch
makepkg -si
```

The version is `0.3.0beta4`. Without a separator before `beta`, pacman sorts
it before `0.3.0rc1` and the final `0.3.0`; `0.3.0_beta.4` would sort after
`0.3.0`.

On a system with XLibre, `xlibre-xserver-xvfb` provides `xorg-server-xvfb`,
and its Xvfb rejects RandR CRTC changes with `BadValue`. `check()` detects
that Xvfb through `pacman -Qo` and skips the nine tests that change
monitors; the other tests run. With Xorg's Xvfb, every test runs.

The Arch package on the [Open Build Service](../obs/README.md#arch-linux-package)
uses the same steps, from the vendored source archives instead of GitHub.

## Publishing to the AUR

The Arch User Repository package is not published yet. To publish this
recipe, or a later one, write `.SRCINFO` next to it and push both to the
package's AUR Git repository:

```sh
makepkg --printsrcinfo > .SRCINFO
```

For a later release, change `_tag`, `pkgver`, and `sha256sums` together,
reset `pkgrel` to 1, install `docs/man/compust.1` and
`docs/man/compust.toml.5` once the tag contains them, and build it again in a clean environment, such as a
clean chroot or an `archlinux:base-devel` container, before publishing.
`namcap PKGBUILD` and `namcap` on the built package should report nothing.
