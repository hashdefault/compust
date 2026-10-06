# Beta testing

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

Compust 0.3.0-beta.2 is the fifth beta, meant for controlled community testing. It is not yet a replacement for picom. Support is declared only where recorded evidence exists; other setups are welcome for testing but remain unqualified.

This beta adds the features of the 1.0 milestone's first step, shadows, fullscreen unredirection, and rules that choose windows by focus, along with rounded corners and configuration reloads when the file is saved, and it fixes two rendering errors. The [release notes](releases/v0.3.0-beta.2.md) list the changes.

## Declared scope

Two recorded machines have run this beta's code: an AMD desktop with integrated Radeon Vega graphics, on Xorg, and an Intel laptop under qtile, which ran the packaged build that adds one fix to the release.

| Area | Recorded with this beta's code | Evidence |
| --- | --- | --- |
| X server | Xorg 21.1.24 in a native session | [Desktop sessions](DESKTOP_TESTING.md#recorded-xorg-sessions-on-the-radeon-vega-desktop-2026-10-05) |
| Window manager | i3 4.25.1 | Same sessions |
| GPU and driver | AMD Ryzen 5 5600GT with Radeon Vega graphics and radeonsi, the modesetting driver, and glamor without TearFree | Same sessions |
| Monitors | One output at 1920×1080 and 60 Hz | Same sessions |
| Rendering | Present and direct XRender, fades, transparency, and blur | Same sessions |
| Features | Shadows, focus rules, rounded corners, and fullscreen suspension, in XRender and in the GPU renderer, without a window manager | [Feature checks](DESKTOP_TESTING.md#recorded-rounded-corners-on-the-radeon-vega-desktop-2026-10-05) |
| Packaged build under qtile | Shadows, transparency, blur, and rounded corners with qtile 0.37.1's borders, in XRender and in the GPU renderer, on an Intel Core i3-1005G1 laptop with Iris Plus graphics and i915, Xorg 21.1.11, and one 1366×768 panel | [Qtile session](DESKTOP_TESTING.md#recorded-qtile-session-on-the-intel-laptop-2026-10-05) |

The desktop sessions ran at `ed2cdb0`, before rounded corners, which are off by default, and before a fix to the GPU renderer, which they did not use. The feature checks ran at `1fc0a1c`, whose compositor source differs from the release's only in its version number. Both used local builds with synthetic windows; none was repeated with the packaged binary. The qtile session ran the Open Build Service package `0.3.0~beta.2+git20261005.3dfd920-1`, the release plus the fix for rounded X borders, in the laptop's usual session; a driver written for it measured captures of synthetic windows, and the probe's scenarios were not run there. Physical hotplug and suspend and resume were not recorded on Xorg.

The RX 9060 XT desktop on XLibre ran 0.3.0-beta.1 under Xmonad, Openbox, i3, and bspwm, and recorded the shadow, focus, and fullscreen fixtures in both renderers before this beta; the Intel laptop's probe sessions, hotplug, and suspend ran the 0.2.0 betas. Their records, listed in the [compatibility matrix](ROADMAP.md#compatibility-matrix), do not cover this beta's code. NVIDIA GPUs, XLibre with this beta, the probe's scenarios on Intel GPUs, window managers other than i3 and qtile, clearly different refresh rates, more than two monitors, and HDR are untested. Reports from any of these setups are especially useful.

## Install

Debian, Ubuntu, Linux Mint, Fedora, and openSUSE Tumbleweed users can install packages instead, as the [README](../README.md#install) describes. Otherwise, download `compust-0.3.0-beta.2-x86_64-linux.tar.gz` and `SHA256SUMS` from the [release page](https://github.com/hashdefault/compust/releases/tag/v0.3.0-beta.2), then verify and unpack them:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.3.0-beta.2-x86_64-linux.tar.gz
cd compust-0.3.0-beta.2-x86_64-linux
./compust --version
```

The binary needs x86_64 Linux with glibc 2.34 or newer. `BUILDINFO` records the source revision, compiler, and build command. To build from source instead, install the pinned Rust toolchain and run:

```sh
git clone --branch v0.3.0-beta.2 https://github.com/hashdefault/compust.git
cd compust
cargo build --release --locked
```

The source build writes `target/release/compust`. Copy either binary to a directory in your `PATH`, such as `~/.local/bin`.

## Run it

Check the server first. `--diagnose` lists the extensions Compust uses and runs safely beside another compositor:

```sh
compust --diagnose
```

Copy the example configuration, adjust it if you like, and validate it:

```sh
mkdir -p ~/.config/compust
cp compust.example.toml ~/.config/compust/compust.toml
compust --check-config
```

Without `--config`, Compust reads `compust/compust.toml` from `$XDG_CONFIG_HOME`, by default `~/.config`, and `--check-config` names the file it found. Stop your current compositor, then start Compust from a terminal in the same X session:

```sh
pkill -x picom
compust
```

If another compositor still owns the screen, Compust refuses to start and says so. To start it with Xmonad, replace your compositor's startup line, such as `spawnOnce "picom ..."`, with the following line:

```haskell
spawnOnce "compust"
```

With i3, use `exec --no-startup-id compust` in its configuration. With `~/.xinitrc`, start `compust &` before the window manager.

Saving the configuration applies it without restarting, once the file has stayed unchanged for a tenth of a second. `pkill -USR1 -x compust` reloads it too, and reads a configuration directory created after Compust started. A file that cannot be read or is invalid is rejected with a warning, and the running settings stay. The [README](../README.md#configuration) describes every setting and the [per-window rules](../README.md#per-window-rules).

### Try the GPU renderer

`backend = "gl"` in the configuration draws with OpenGL ES on the X server's GPU instead of XRender. It is opt-in and recorded on two AMD desktops with radeonsi: the RX 9060 XT on XLibre with 0.3.0-beta.1, and the Radeon Vega on Xorg with this beta's feature checks, rounded corners included. It also ran on the Intel laptop's Iris Plus graphics with the packaged build [under qtile](DESKTOP_TESTING.md#recorded-qtile-session-on-the-intel-laptop-2026-10-05), without a fallback. Where it cannot start, or when a frame fails, Compust logs a warning and uses XRender for the rest of the session. Reports from other Intel GPUs and from NVIDIA drivers are especially useful; include the log lines that name the device or the fallback.

## Return to your previous compositor

Stop Compust with SIGTERM, or with Ctrl+C in its terminal. It releases the screen and exits successfully. Then start your previous compositor again, for example:

```sh
pkill -TERM -x compust
picom -b
```

Restore any startup line you changed. Logging out also ends Compust: when the X server closes, Compust exits with `The X11 server closed the connection`, which is expected.

## Known limits

- Fullscreen unredirection needs one opaque, square window over the whole screen, so it never applies to one monitor of several.
- Shadows are black, and shaped windows cast none and keep square corners. Rounded X borders use the top-left border pixel as their color around every corner; window managers that paint different colors on different edges cannot keep those colors distinct at the corners.
- There are no movement or scale animations, no global settings for the opacity of active and inactive windows beyond rules, and no picom configuration compatibility.
- Rules match exact text. Focus comes from `_NET_ACTIVE_WINDOW`; under a window manager that does not set it, such as Xmonad without `XMonad.Hooks.EwmhDesktops`, every window counts as focused.
- A blurred window blurs its whole background again whenever something beneath it changes. `blur_radius` rounds to 2, 4, 8, or 16 pixels.
- The GPU renderer is recorded on two AMD desktops and one Intel laptop. On the desktops it draws the same frames as XRender within two levels of color, uses about 62 MiB more memory, and costs more than XRender while windows are resized. On the laptop its captures matched within two levels, it used about 73 MiB more, and that memory stayed after a reload back to XRender.
- One process composes one X screen. Multiple monitors share one surface, and Present follows the timing of one monitor.
- Present copies a single buffer. Whether output tears depends on the driver: XLibre's modesetting driver enables TearFree by default, and Xorg 21.1.24's has none.

## Report a problem

Collect the version, diagnostics, and monitor layout:

```sh
compust --version
compust --diagnose
xrandr --current
```

Note your distribution, X server and version, window manager and version, GPU and driver (for example from `lspci -k`), and your configuration file. Reproduce the problem with debug logging:

```sh
RUST_LOG=compust=debug compust 2>compust.log
```

Describe the steps, the expected and actual result, and whether stopping Compust restored the desktop. Then open a [bug report](https://github.com/hashdefault/compust/issues/new?template=bug.yml) with that information. Remove private details from logs and screenshots first.
