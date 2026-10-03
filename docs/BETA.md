# Beta testing

[English (US)](BETA.md) | [Português (Brasil)](BETA.pt-BR.md)

Compust 0.2.0-beta.1 is the first beta, meant for controlled community testing. It is not yet a replacement for picom. Support is declared only where recorded evidence exists; other setups are welcome for testing but remain unqualified.

## Declared scope

| Area | Qualified for this beta | Evidence |
| --- | --- | --- |
| X server | XLibre 25.1.9 in a native session | [Hardware desktop session](DESKTOP_TESTING.md#recorded-hardware-desktop-session-2026-10-03) |
| Window manager | Xmonad 0.18.1 with EWMH | Same session |
| GPU and driver | AMD Radeon Vega (Ryzen 5 5600GT) with the modesetting driver and glamor, Mesa 26.2.4 | Same session |
| Monitors | One or two outputs at 1920×1080 and 60 Hz, including unplugging and reconnecting | [Monitor session](DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03) |
| Rendering | Present and direct XRender, fades, transparency, and blur | [Pyramid blur session](DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03) |

Xorg 21.1 and XLibre 25.1 also pass the desktop scenarios nested in Xephyr, which covers protocol behavior but not drivers or displays. Intel and NVIDIA GPUs, other window managers, Xorg on hardware, mixed refresh rates, and HDR are untested. Reports from those setups are especially useful.

## Install

Download `compust-0.2.0-beta.1-x86_64-linux.tar.gz` and `SHA256SUMS` from the [release page](https://github.com/hashdefault/compust/releases/tag/v0.2.0-beta.1), then verify and unpack them:

```sh
sha256sum -c SHA256SUMS
tar -xzf compust-0.2.0-beta.1-x86_64-linux.tar.gz
cd compust-0.2.0-beta.1-x86_64-linux
./compust --version
```

The binary needs x86_64 Linux with glibc 2.34 or newer. `BUILDINFO` records the source revision, compiler, and build command. To build from source instead, install the pinned Rust toolchain and run:

```sh
git clone --branch v0.2.0-beta.1 https://github.com/hashdefault/compust.git
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
compust --check-config --config ~/.config/compust/compust.toml
```

Compust reads a configuration only from `--config`; the path above is a convention. Stop your current compositor, then start Compust from a terminal in the same X session:

```sh
pkill -x picom
compust --config ~/.config/compust/compust.toml
```

If another compositor still owns the screen, Compust refuses to start and says so. To start it with Xmonad, replace your compositor's startup line, such as `spawnOnce "picom ..."`, with the following line:

```haskell
spawnOnce "compust --config $HOME/.config/compust/compust.toml"
```

With `~/.xinitrc`, start `compust --config ~/.config/compust/compust.toml &` before the window manager.

## Return to your previous compositor

Stop Compust with SIGTERM, or with Ctrl+C in its terminal. It releases the screen and exits successfully. Then start your previous compositor again, for example:

```sh
pkill -TERM -x compust
picom -b
```

Restore any startup line you changed. Logging out also ends Compust: when the X server closes, Compust exits with `The X11 server closed the connection`, which is expected.

## Known limits

- Every damage event repaints the full screen. An idle desktop stays idle, but large or busy screens cost more.
- Each translucent window blurs its own area, so many overlapping translucent windows multiply that work. `blur_radius` rounds to 2, 4, 8, or 16 pixels.
- Fullscreen windows are still composited; there is no unredirection for games or video.
- There are no shadows, rounded corners, movement or scale animations, per-window rules, configuration reload, or picom configuration compatibility.
- One process composes one X screen. Multiple monitors share one surface, and Present follows the timing of one monitor.
- Present copies a single buffer. Whether output tears depends on the driver; XLibre's modesetting driver enabled TearFree by default on the recorded machine.

## Report a problem

Collect the version, diagnostics, and monitor layout:

```sh
compust --version
compust --diagnose
xrandr --current
```

Note your distribution, X server and version, window manager and version, GPU and driver (for example from `lspci -k`), and your configuration file. Reproduce the problem with debug logging:

```sh
RUST_LOG=compust=debug compust --config ~/.config/compust/compust.toml 2>compust.log
```

Describe the steps, the expected and actual result, and whether stopping Compust restored the desktop. Then open a [bug report](https://github.com/hashdefault/compust/issues/new?template=bug.yml) with that information. Remove private details from logs and screenshots first.
