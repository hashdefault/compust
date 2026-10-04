# Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

[Project website and documentation](https://hashdefault.github.io/compust/)

Compust is an experimental, standalone **X11 compositor written in Rust**, targeting Xorg and XLibre. Its goal is a small, understandable alternative to picom, with smooth animations, background blur, and transparency.

A compositor combines application windows into the final desktop image. Compust runs **on an existing X server**, alongside your window manager. It does not start or replace Xorg/XLibre, manage window placement, or provide a native Wayland session.

**Status: third beta, version 0.2.0-beta.3, for controlled testing.** The XRender backend has automated pixel tests on Xvfb and recorded qualification on two machines: an AMD desktop running XLibre with Xmonad, and an Intel laptop running Xorg with Xmonad, Openbox, and i3; other drivers, servers, and window managers still need community testing. Compust is not yet a drop-in replacement for picom, and no performance advantage over picom has been demonstrated. The [beta guide](docs/BETA.md) explains how to install it, return to your previous compositor, and report problems; the [roadmap](docs/ROADMAP.md) lists the remaining work.

## What works today

Opening and closing windows use a smoothstep fade, including closing a window halfway through its opening animation. Transparency combines an application's ARGB content, `_NET_WM_WINDOW_OPACITY`, and the configured global opacity. Translucent windows can blur the content behind them. The blur repeatedly halves the area behind a window with bilinear sampling and scales it back up, which GPU-accelerated servers keep on the GPU.

Compust tracks window stacking, movement, resizing, bounding shapes, redraws, and root wallpaper pixmaps. It retains named pixmaps during closing animations. The overlay has an empty input region so clicks reach the applications below it. An existing compositor is never replaced automatically.

| Extension or convention | Current behavior |
| --- | --- |
| Composite 0.4+ | Required: manual redirection, named window pixmaps, overlay |
| Damage 1.0+ | Required: redraw notifications; no continuous repaint on an idle desktop |
| Render 0.11+ | Required: composition, alpha masks, transforms; bilinear filtering enables blur |
| XFixes 2.0+ and Shape 1.1+ | Required: input-transparent overlay and shaped windows |
| Present | Optional: copy presentation, waiting for completion and buffer-idle events |
| RandR | Optional: screen-change subscription and buffer recreation; physical hotplug recorded on one AMD/XLibre desktop and one Intel/Xorg laptop |
| EWMH / ICCCM | Compositor selection, manager announcement, opacity, and client discovery through `WM_STATE` |
| Root wallpaper | `_XROOTPMAP_ID`, then `ESETROOT_PMAP_ID`; dark fallback when neither is usable |
| DRI3 1.2 and Sync 3.1 | Optional: the GPU renderer shares the back buffer, window, and wallpaper pixmaps through DRI3, and a Sync fence makes the server send its GPU work before each frame; no explicit synchronization |

“Modern X11 support” is an incremental compatibility goal, not a promise to implement every extension. Present availability does not establish tear-free behavior on every driver. The XRender fallback is not synchronized to vblank.

If a Present submission is rejected with `BadMatch` and the original rendering buffers remain valid, Compust logs a warning and uses direct XRender copying until restart. After a RandR change, Compust rebuilds its buffers without waiting for a pending presentation, because a monitor reconfiguration on AMD hardware discarded its completion events. If Present reports nothing for a submission within one second, Compust likewise rebuilds its buffers and repaints, so a lost notification cannot freeze the screen. Other protocol errors retain their existing handling.

## Build and run

You need Linux, Rust 1.95.0 (the toolchain file selects it), Cargo, a C linker, and an X server with the required extensions. With a rustup installation, make sure `~/.cargo/bin` is in your `PATH`. The X11 connection uses `x11rb`'s Rust implementation; Compust does not require Xlib development headers. Beta testers can download a checked binary instead, as the [beta guide](docs/BETA.md) explains.

```sh
git clone https://github.com/hashdefault/compust.git
cd compust
cargo build --release --locked
./target/release/compust --help
./target/release/compust --diagnose
./target/release/compust --config compust.example.toml
```

Run it from your X11 session after stopping the compositor already serving that screen. `--diagnose` only inspects extensions and can run while another compositor is active. Use `--display :1` to select a different server; otherwise `DISPLAY` and the usual Xauthority authentication are used. Stop with Ctrl+C or SIGTERM. Compust does not install autostart entries or modify your desktop configuration.

For isolated manual experiments, start a nested server, then run Compust and an X11 application on that display:

```sh
Xephyr :99 -screen 1280x720 -ac -nolisten tcp &
DISPLAY=:99 ./target/release/compust --config compust.example.toml
# In another terminal:
DISPLAY=:99 xterm
```

Choose an unused display number. This example disables X authentication only for the local test server; it does not expose a TCP listener. Xephyr and xterm are separate packages. The automated test suite uses Xvfb and allocates display numbers automatically.

## Configuration

The example is a complete configuration. Without `--config`, Compust reads the first `compust/compust.toml` found in `$XDG_CONFIG_HOME` (by default `~/.config`), then in each directory of `$XDG_CONFIG_DIRS` (by default `/etc/xdg`); with no file, built-in defaults apply. Unknown fields and out-of-range values produce an error before connecting to X11.

Send SIGUSR1 to reload the configuration without restarting, for example with `pkill -USR1 -x compust`. A reload reads the file a restart would read. If that file cannot be read or is invalid, Compust logs a warning and keeps its current settings. `opacity` and `max_fps` apply from the next frame. A new `fade_ms` applies to every opening and closing that starts afterward, including for windows already open; fades in progress finish with their previous duration. A change to `blur_radius`, `vsync`, or `backend` replaces the renderer once the frame being presented is done.

```toml
opacity = 100
fade_ms = 180
blur_radius = 4
max_fps = 120
vsync = true
backend = "xrender"
```

| Setting | Meaning |
| --- | --- |
| `opacity` | Global opacity percentage, 0–100, multiplied by application opacity |
| `fade_ms` | Opening/closing duration in milliseconds, 0–65535; zero disables fades |
| `blur_radius` | Approximate blur radius in pixels, 0–16, rounded to 2, 4, 8, or 16; zero disables blur |
| `max_fps` | Repaint ceiling, 1–1000; not a promise of actual frame rate |
| `vsync` | Use Present if available; `false` selects direct XRender copying |
| `backend` | `"xrender"` draws through the X server; `"gl"` draws with OpenGL ES on the server's GPU, and falls back to XRender with a warning where it cannot |

Blur applies behind translucent or ARGB windows. If the server has no bilinear filter, Compust logs a warning and runs without blur. `max_fps` does not force idle repaints; the event loop wakes at most once per second while idle to observe shutdown and reload signals.

```sh
./target/release/compust --check-config --config compust.example.toml
./target/release/compust --check-config  # names the file it would read
RUST_LOG=compust=debug ./target/release/compust
```

## Testing and contributing

Install Xvfb (`xvfb` on Debian/Ubuntu, `xorg-server-xvfb` on Arch), then run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

Each integration scenario starts its own Xvfb and a real Compust process. Tests observe rendered pixels and protocol behavior rather than mocking the server. Set `XVFB=/path/to/Xvfb` to use a nonstandard binary. `COMPUST_ARTIFACTS=artifacts cargo test --test x11` saves selected scenes as PPM images for inspection. The GPU crate's tests draw on Mesa's software EGL device and skip without one; `COMPUST_GPU_TESTS=1`, as in CI, makes that a failure. `COMPUST_GPU_DISPLAY=:0 cargo test --test gpu` checks DRI3 sharing against a real server's GPU with offscreen pixmaps only.

Contributions in **English or Brazilian Portuguese** are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture](docs/ARCHITECTURE.md), or the [roadmap](docs/ROADMAP.md). Driver reports, reproducible failures, documentation, and performance measurements are useful contributions too.

## Current limits

This prototype repaints only the area of the screen that changed, and each blurred window keeps its blurred background until something beneath it changes, which blurs it again across its whole footprint. On the [recorded AMD/XLibre desktop](docs/DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03), a full-screen translucent window with blur kept 60 frames per second while Xorg used about 4% of a core. Windows hidden behind opaque ones are not painted. The opt-in GPU renderer ([recorded on one desktop](docs/DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04)) draws the same frames as XRender within two levels of color, at about the same total CPU there, more when windows are resized, and with about 62 MiB more memory for the GL driver. The [next milestone](docs/ROADMAP.md#next-measure-and-reduce-rendering-work) starts with benchmarks; its [first records](docs/DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03) compare Compust with picom on one machine. It has no shadows, rounded corners, movement/scale animations, per-window rules, fullscreen unredirection, or picom configuration compatibility.

The planned [Window Animations milestone](docs/ROADMAP.md#window-animations-planned) extends the existing fade with pop, slide, easing, and per-window rules. Its configuration examples describe future work and are not accepted by the current binary.

One process handles one X screen; a multi-monitor root is composed as one surface. Mixed-refresh scheduling, HDR/color management, VRR, DMA-BUF import, explicit synchronization, and XLibre-specific extensions are not implemented or certified. Physical hotplug is verified only on the [recorded AMD/XLibre desktop](docs/DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03) and the [recorded Intel/Xorg laptop](docs/DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03). Native Wayland support is outside the current scope.

## Background and license

The project is inspired by the standalone compositor model of [picom](https://github.com/yshui/picom), whose lineage includes Compton. Picom already provides animations and effects; Compust's intended contribution is an approachable Rust implementation with measured improvements over time. This repository is a new implementation, not a port of picom's source code.

Protocol references: [x11rb](https://docs.rs/x11rb/0.13.2/x11rb/), [EWMH compositing managers](https://specifications.freedesktop.org/wm/latest/ar01s08.html), and [XLibre](https://github.com/X11Libre/xserver). Licensed under [MIT](LICENSE).
