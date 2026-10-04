# Beta testing

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

Compust 0.3.0-beta.1 is the fourth beta, meant for controlled community testing. It is not yet a replacement for picom. Support is declared only where recorded evidence exists; other setups are welcome for testing but remain unqualified.

This beta paints differently from the 0.2.0 betas: it repaints only the area that changed, keeps each blurred background until something beneath it changes, and skips windows hidden behind opaque ones. It also adds configuration discovery and reload, per-window rules, blur weighted by each pixel's opacity, and an opt-in GPU renderer. The [release notes](releases/v0.3.0-beta.1.md) list the changes.

## Declared scope

One recorded machine has run this beta: an AMD desktop with a Radeon RX 9060 XT, on XLibre.

| Area | Recorded with this beta's source | Evidence |
| --- | --- | --- |
| X server | XLibre 25.1.9 in a native session | [Desktop sessions](DESKTOP_TESTING.md#recorded-desktop-sessions-on-the-rx-9060-xt-2026-10-04) |
| Window manager | Xmonad 0.18.1, Openbox 3.6.1, i3 4.25.1, or bspwm 0.9.12 | Same sessions |
| GPU and driver | AMD Radeon RX 9060 XT with radeonsi in Mesa 26.2.4, the modesetting driver, and glamor | Same sessions |
| Monitors | One output at 1920×1080 and 60 Hz | Same sessions |
| Rendering | Present and direct XRender, fades, transparency, and blur | Same sessions |

Those sessions ran the release's compositor source in a local build with synthetic windows; none was repeated with the packaged binary.

Earlier commits of this beta's rendering code also ran benchmark scenes on that machine under dwm 6.8, on one and two monitors: [region repaint](DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04), [blur reuse](DESKTOP_TESTING.md#recorded-blur-reuse-2026-10-04), [occlusion](DESKTOP_TESTING.md#recorded-occlusion-2026-10-04), and the opt-in [GPU renderer](DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04) beside XRender. A [reload session](DESKTOP_TESTING.md#recorded-reload-session-2026-10-03), recorded before the rendering changes, covers configuration reloads while an output turns on and off and the layout changes. Physical hotplug and suspend and resume were not recorded on this machine. The per-window rules have automated tests only; the weighted blur was also checked by eye on Brave's menus.

The two machines qualified for 0.2.0-beta.3 have not run this beta: the AMD Radeon Vega desktop with Xmonad, and the Intel Iris Plus laptop with Xmonad, Openbox, and i3. Their records, listed in the [compatibility matrix](ROADMAP.md#compatibility-matrix), cover window management, physical hotplug, and suspend and resume with the earlier painter. NVIDIA GPUs, other window managers, Xorg with AMD, XLibre with Intel, clearly different refresh rates, more than two monitors, and HDR are untested. Reports from any of these setups are especially useful.

## Install

Download `compust-0.3.0-beta.1-x86_64-linux.tar.gz` and `SHA256SUMS` from the [release page](https://github.com/hashdefault/compust/releases/tag/v0.3.0-beta.1), then verify and unpack them:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.3.0-beta.1-x86_64-linux.tar.gz
cd compust-0.3.0-beta.1-x86_64-linux
./compust --version
```

The binary needs x86_64 Linux with glibc 2.34 or newer. `BUILDINFO` records the source revision, compiler, and build command. To build from source instead, install the pinned Rust toolchain and run:

```sh
git clone --branch v0.3.0-beta.1 https://github.com/hashdefault/compust.git
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

With `~/.xinitrc`, start `compust &` before the window manager.

After editing the configuration, apply it without restarting:

```sh
pkill -USR1 -x compust
```

A file that cannot be read or is invalid is rejected with a warning, and the running settings stay. The [README](../README.md#configuration) describes every setting and the [per-window rules](../README.md#per-window-rules).

### Try the GPU renderer

`backend = "gl"` in the configuration draws with OpenGL ES on the X server's GPU instead of XRender. It is opt-in and recorded on one desktop, with radeonsi. Where it cannot start, or when a frame fails, Compust logs a warning and uses XRender for the rest of the session. Reports from Intel and NVIDIA drivers are especially useful; include the log lines that name the device or the fallback.

## Return to your previous compositor

Stop Compust with SIGTERM, or with Ctrl+C in its terminal. It releases the screen and exits successfully. Then start your previous compositor again, for example:

```sh
pkill -TERM -x compust
picom -b
```

Restore any startup line you changed. Logging out also ends Compust: when the X server closes, Compust exits with `The X11 server closed the connection`, which is expected.

## Known limits

- Fullscreen windows are still composited; there is no unredirection for games or video.
- There are no shadows, rounded corners, movement or scale animations, or picom configuration compatibility.
- Rules match exact text and cannot choose windows by focus.
- A blurred window blurs its whole background again whenever something beneath it changes. `blur_radius` rounds to 2, 4, 8, or 16 pixels.
- The GPU renderer is recorded on one desktop. There it draws the same frames as XRender within two levels of color, uses about 62 MiB more memory, and costs more than XRender while windows are resized.
- One process composes one X screen. Multiple monitors share one surface, and Present follows the timing of one monitor.
- Present copies a single buffer. Whether output tears depends on the driver; XLibre's modesetting driver enabled TearFree by default on the recorded Radeon Vega machine.

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
