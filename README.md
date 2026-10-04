# Compust

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

[Project website and documentation](https://hashdefault.github.io/compust/)

Compust is an experimental, standalone **X11 compositor written in Rust**, targeting Xorg and XLibre. Its goal is a small, understandable alternative to picom, with smooth animations, background blur, and transparency.

A compositor combines application windows into the final desktop image. Compust runs **on an existing X server**, alongside your window manager. It does not start or replace Xorg/XLibre, manage window placement, or provide a native Wayland session.

**Status: fourth beta, version 0.3.0-beta.1, for controlled testing.** Automated pixel tests run on Xvfb, and sessions are recorded on three machines. An AMD desktop with a Radeon RX 9060 XT on XLibre is the only one that has run this beta, under Xmonad, Openbox, i3, bspwm, and dwm; another AMD desktop on XLibre and an Intel laptop on Xorg were recorded with the earlier betas. Other drivers, servers, and window managers still need community testing. Compust is not yet a drop-in replacement for picom, and no performance advantage over picom has been demonstrated. The [beta guide](docs/BETA.md) explains how to install it, return to your previous compositor, and report problems. The [milestones](#milestones-and-verified-environments) below show what is done, and the [1.0 milestone](docs/ROADMAP.md#10-stable-release) of the [roadmap](docs/ROADMAP.md) defines what a stable release still requires.

## Milestones and verified environments

Each row links to its record.

| Milestone | State | What it established |
| --- | --- | --- |
| [0.1 foundation](docs/ROADMAP.md#01-foundation-implemented) | Done | XRender composition, fades, transparency, blur, shapes, and stacking, checked by pixel tests against a real X server |
| [First beta](docs/ROADMAP.md#first-beta-four-steps) | Published as 0.2.0-beta.1, beta.2, and beta.3 | Four gates met for the declared scope: window stability, monitors and resources, real desktops, and a checked release archive |
| [Configuration reload and rules](docs/ROADMAP.md#everyday-usability) | Released in 0.3.0-beta.1 | Configuration discovery, reload with SIGUSR1, per-window rules, and blur weighted by each pixel's opacity |
| [Rendering work](docs/ROADMAP.md#rendering-work-four-steps-done-on-one-desktop) | Released in 0.3.0-beta.1; measured on one desktop | Benchmark scenes against picom, stacking tracked without tree queries, region repaint, blur reuse, and occlusion |
| [GPU renderer](docs/ROADMAP.md#rendering-backend-and-protocol-expansion) | Opt-in since 0.3.0-beta.1; recorded on one desktop | OpenGL ES through DRI3, drawing the same frames as XRender within two levels of color; XRender stays the default and the fallback |
| [1.0 stable release](docs/ROADMAP.md#10-stable-release) | Current milestone; [shadows](docs/ROADMAP.md#10-step-1-shadows) done on `main` | Fullscreen unredirection, rules by focus, NVIDIA and more desktops, a picom comparison per GPU vendor, endurance runs, outside testers, and packaging |

| Environment | Recorded there |
| --- | --- |
| Xvfb in CI, on every push | 106 X11 tests that run a real Compust process against a real server and check pixels and protocol behavior, with 35 unit tests, 6 command-line tests, and the GPU crate's 9 tests on Mesa's software device |
| AMD Ryzen 5 5600GT (Radeon Vega) desktop: XLibre 25.1.9, Xmonad 0.18.1, one and two 1920×1080 monitors | [Desktop scenarios](docs/DESKTOP_TESTING.md#recorded-hardware-desktop-session-2026-10-03) with fades, translucency, and [blur](docs/DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03); [mode and layout changes, and both cables unplugged and reconnected](docs/DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03) |
| Intel Core i3-1005G1 (Iris Plus) laptop: Xorg 21.1.11, with Xmonad 0.17.2, Openbox 3.6.1, and i3 4.23 | Desktop scenarios under [Xmonad](docs/DESKTOP_TESTING.md#recorded-intelxorg-sessions-2026-10-03), [Openbox](docs/DESKTOP_TESTING.md#recorded-openbox-sessions-2026-10-03), and [i3](docs/DESKTOP_TESTING.md#recorded-i3-sessions-2026-10-03); an [external display unplugged and reconnected](docs/DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03); [suspend and resume](docs/DESKTOP_TESTING.md#recorded-suspend-and-resume-2026-10-03) |
| AMD Ryzen 5 5600X with a Radeon RX 9060 XT: XLibre 25.1.9, with Xmonad 0.18.1, Openbox 3.6.1, i3 4.25.1, bspwm 0.9.12, and dwm 6.8, one and two 1920×1080 monitors | [Desktop scenarios](docs/DESKTOP_TESTING.md#recorded-desktop-sessions-on-the-rx-9060-xt-2026-10-04) with 0.3.0-beta.1's source under Xmonad, Openbox, i3, and bspwm, with fades, translucency, and blur. Under dwm: [configuration reloads](docs/DESKTOP_TESTING.md#recorded-reload-session-2026-10-03); [benchmark scenes against picom v13](docs/DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03); [region repaint](docs/DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04), [blur reuse](docs/DESKTOP_TESTING.md#recorded-blur-reuse-2026-10-04), [occlusion](docs/DESKTOP_TESTING.md#recorded-occlusion-2026-10-04), and the [GPU renderer](docs/DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04) |
| Xephyr nested in Xvfb: Xorg 21.1.24 and XLibre 25.1.9, with Xmonad 0.18.1 | [Desktop scenarios](docs/DESKTOP_TESTING.md#recorded-baseline-2026-10-03) with Present, with direct XRender, and with [effects](docs/DESKTOP_TESTING.md#recorded-effects-baseline-2026-10-03); protocol behavior only, without a driver or a display |

Most hardware sessions use the probe's synthetic windows instead of applications, and the Radeon Vega and Intel machines were recorded before the rendering work. The rules have automated tests but no hardware record yet; the weighted blur was also checked by eye on Brave's menus. NVIDIA drivers, Xorg on AMD, XLibre on Intel, other window managers, clearly different refresh rates, more than two monitors, and HDR are untested. The [compatibility matrix](docs/ROADMAP.md#compatibility-matrix) gives each record's versions and limits.

## What works today

Opening and closing windows use a smoothstep fade, including closing a window halfway through its opening animation. Transparency combines an application's ARGB content, `_NET_WM_WINDOW_OPACITY`, and the configured global opacity. Translucent windows can blur the content behind them. The blur repeatedly halves the area behind a window with bilinear sampling and scales it back up, which GPU-accelerated servers keep on the GPU. Windows can cast soft [shadows](#shadows), which stay off until `shadow_radius` is set. [Per-window rules](#per-window-rules) set the opacity, blur, fade duration, and shadow of windows chosen by class, type, or title.

Compust tracks window stacking, movement, resizing, bounding shapes, redraws, and root wallpaper pixmaps. It retains named pixmaps during closing animations. The overlay has an empty input region so clicks reach the applications below it. An existing compositor is never replaced automatically.

| Extension or convention | Current behavior |
| --- | --- |
| Composite 0.4+ | Required: manual redirection, named window pixmaps, overlay |
| Damage 1.0+ | Required: redraw notifications; no continuous repaint on an idle desktop |
| Render 0.11+ | Required: composition, alpha masks, transforms; bilinear filtering enables blur |
| XFixes 2.0+ and Shape 1.1+ | Required: input-transparent overlay and shaped windows |
| Present | Optional: copy presentation, waiting for completion and buffer-idle events |
| RandR | Optional: screen-change subscription and buffer recreation; physical hotplug recorded on one AMD/XLibre desktop and one Intel/Xorg laptop |
| EWMH / ICCCM | Compositor selection, manager announcement, opacity, client discovery through `WM_STATE`, and rules matching `WM_CLASS`, `_NET_WM_WINDOW_TYPE` (with `WM_TRANSIENT_FOR` for its default), and `_NET_WM_NAME` or `WM_NAME`; `_GTK_FRAME_EXTENTS` marks windows that draw their own shadow |
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

Send SIGUSR1 to reload the configuration without restarting, for example with `pkill -USR1 -x compust`. A reload reads the file a restart would read. If that file cannot be read or is invalid, Compust logs a warning and keeps its current settings. `opacity`, `max_fps`, the shadow settings, and the opacity, blur, and shadow that rules give apply from the next frame, to windows already open too. A new `fade_ms`, global or in a rule, applies to every opening and closing that starts afterward, including for windows already open; fades in progress finish with their previous duration. A change to `blur_radius`, `vsync`, or `backend` replaces the renderer once the frame being presented is done.

```toml
opacity = 100
fade_ms = 180
blur_radius = 4
shadow_radius = 0
shadow_offset_x = 0
shadow_offset_y = 0
shadow_opacity = 50
max_fps = 120
vsync = true
backend = "xrender"
```

| Setting | Meaning |
| --- | --- |
| `opacity` | Global opacity percentage, 0–100, multiplied by application opacity |
| `fade_ms` | Opening/closing duration in milliseconds, 0–65535; zero disables fades |
| `blur_radius` | Approximate blur radius in pixels, 0–16, rounded to 2, 4, 8, or 16; zero disables blur |
| `shadow_radius` | How far a shadow spreads beyond its window in pixels, 0–64; zero, the default, draws no shadows |
| `shadow_offset_x`, `shadow_offset_y` | Where a shadow lies from its window in pixels, −64 to 64: right and down, or left and up when negative |
| `shadow_opacity` | How dark a shadow is where it is darkest, as a percentage, 0–100, multiplied by its window's opacity |
| `max_fps` | Repaint ceiling, 1–1000; not a promise of actual frame rate |
| `vsync` | Use Present if available; `false` selects direct XRender copying |
| `backend` | `"xrender"` draws through the X server; `"gl"` draws with OpenGL ES on the server's GPU, and falls back to XRender with a warning where it cannot |

Blur applies behind translucent or ARGB windows, as strongly as each of the window's pixels is opaque: the transparent shadow margin around a browser's menu gets almost none, and the blur fades in and out with the window. If the server has no bilinear filter, Compust logs a warning and runs without blur. `max_fps` does not force idle repaints; the event loop wakes at most once per second while idle to observe shutdown and reload signals.

### Shadows

Shadows are on `main` and will be in the next beta; 0.3.0-beta.1 rejects their settings. With `shadow_radius` above zero, windows cast a black shadow: the window's rectangle, moved by the offset and blurred so that it fades out over the radius. A shadow lies around its window, never beneath it, so a translucent window is no darker for its own shadow, and it fades in and out with the window.

A window casts a shadow when its type is `normal`, `dialog`, `utility`, `splash`, or `toolbar`. Desktops, docks, menus, tooltips, notifications, combo boxes, and drag icons cast none. Neither does a window that names no type and that the window manager leaves alone, such as a status bar or the menu of an older toolkit, nor one that declares margins for a shadow of its own in `_GTK_FRAME_EXTENTS`, as client-side-decorated GTK windows do. `shadow` in a [rule](#per-window-rules) decides for the windows it chooses, either way. A window with a non-rectangular shape never casts one, because the shadow would be that of its bounding rectangle.

Under a tiling window manager each window's shadow falls on its neighbors. A rule with `window_type = "normal"` and `shadow = false` leaves shadows to dialogs and other floating windows that name their type.

### Per-window rules

Each `[[rules]]` table chooses windows and changes settings for them. Rules go after the global settings, because TOML puts every table after the plain keys.

```toml
# Terminals at 90% opacity.
[[rules]]
wm_class = "Alacritty"
opacity = 90

# Tooltips without blur behind them, and without fades.
[[rules]]
window_type = "tooltip"
blur = false
fade_ms = 0

# Notifications with a shadow, which their type would leave out.
[[rules]]
window_type = "notification"
shadow = true
```

| Field | Meaning |
| --- | --- |
| `wm_class` | Chooses windows whose resource class, the second string of `WM_CLASS`, is this text |
| `window_type` | Chooses windows of this EWMH type: `desktop`, `dock`, `toolbar`, `menu`, `utility`, `splash`, `dialog`, `dropdown_menu`, `popup_menu`, `tooltip`, `notification`, `combo`, `dnd`, or `normal` |
| `name` | Chooses windows whose title, `_NET_WM_NAME` or else `WM_NAME`, is this text |
| `opacity` | Opacity percentage, 0–100, used instead of the global `opacity` and multiplied by application opacity |
| `blur` | `false` keeps the content behind the window sharp; `true` blurs it as by default, while `blur_radius` is above zero |
| `fade_ms` | Opening/closing duration in milliseconds, 0–65535, used instead of the global `fade_ms` |
| `shadow` | `false` gives the window no shadow; `true` gives it one whatever its type or margins, while `shadow_radius` is above zero |

A rule needs at least one of the first three fields, which choose windows, and at least one of the last four, which it sets. Text must match exactly, including case, and a window must match every field a rule chooses by. Each setting comes from the first matching rule that sets it, so specific rules go before broad ones; `true` in a rule thus keeps blur for windows a later rule turns it off for. Run `xprop` and click a window to see its `WM_CLASS`, `_NET_WM_WINDOW_TYPE`, and `_NET_WM_NAME`.

Compust reads these properties from the application's window inside the window manager's frame, and reads them again when they change, such as when a title changes. A window that names no type Compust knows is `dialog` when it is transient for another window and the window manager handles it, and `normal` otherwise, as EWMH specifies. A property that is missing or malformed matches no text. A closing window keeps the rules it had while it fades out.

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

This prototype repaints only the area of the screen that changed, and each blurred window keeps its blurred background until something beneath it changes, which blurs it again across its whole footprint. On the [recorded AMD/XLibre desktop](docs/DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03), a full-screen translucent window with blur kept 60 frames per second while Xorg used about 4% of a core. Windows hidden behind opaque ones are not painted. The opt-in GPU renderer ([recorded on one desktop](docs/DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04)) draws the same frames as XRender within two levels of color, at about the same total CPU there, more when windows are resized, and with about 62 MiB more memory for the GL driver. The [rendering milestone](docs/ROADMAP.md#rendering-work-four-steps-done-on-one-desktop) measured these costs; its [first records](docs/DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03) compare Compust with picom on one machine, before the repaint changes. Shadows are black and rectangular, and shaped windows cast none; they have pixel tests with XRender on Xvfb and a test of their GPU draw, but no hardware session has recorded them yet. Compust has no rounded corners, movement/scale animations, fullscreen unredirection, or picom configuration compatibility; the [1.0 milestone](docs/ROADMAP.md#10-stable-release) adds fullscreen unredirection.

The planned [Window Animations milestone](docs/ROADMAP.md#window-animations-planned) extends the existing fade with pop, slide, and easing, chosen per window. Its configuration examples describe future work and are not accepted by the current binary.

One process handles one X screen; a multi-monitor root is composed as one surface. Mixed-refresh scheduling, HDR/color management, VRR, DMA-BUF import, explicit synchronization, and XLibre-specific extensions are not implemented or certified. Physical hotplug is verified only on the [recorded AMD/XLibre desktop](docs/DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03) and the [recorded Intel/Xorg laptop](docs/DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03). Native Wayland support is outside the current scope.

## Background and license

The project is inspired by the standalone compositor model of [picom](https://github.com/yshui/picom), whose lineage includes Compton. Picom already provides animations and effects; Compust's intended contribution is an approachable Rust implementation with measured improvements over time. This repository is a new implementation, not a port of picom's source code.

Protocol references: [x11rb](https://docs.rs/x11rb/0.13.2/x11rb/), [EWMH compositing managers](https://specifications.freedesktop.org/wm/latest/ar01s08.html), and [XLibre](https://github.com/X11Libre/xserver). Licensed under [MIT](LICENSE).
