# Desktop qualification

[English (US)](DESKTOP_TESTING.md) | [Português (Brasil)](DESKTOP_TESTING.pt-BR.md)

This guide covers the reproducible desktop work in beta step 3. Xmonad running inside Xephyr exercises a real window manager and X server. Xephyr hosted by Xvfb uses software rendering; these results do not qualify a GPU driver, physical monitor, or tear-free scanout. Recorded AMD/XLibre sessions below cover physical monitor transitions for step 2 and the probe's desktop scenarios on hardware for step 3. A recorded Intel/Xorg laptop covers the same scenarios and a mode change on its single panel. Other hardware, window managers, and actual applications remain open.

## Run the isolated baseline

Install the pinned Rust toolchain, a C linker, GHC with the `xmonad` and `xmonad-contrib` libraries, Xvfb, Xephyr, `xprop`, `xdpyinfo`, `xrandr`, and standard Linux utilities including `timeout`, `getconf`, and `sha256sum`. Run from the repository root. The runner allocates both displays automatically and starts a private Xmonad configuration; it can run from a Wayland session or without a desktop. Set `WINDOW_MANAGER` to `openbox`, `i3`, or `bspwm` to test one of those instead, each with its own private configuration ([Openbox](../tools/desktop/openbox.xml), [i3](../tools/desktop/i3.config), [bspwm](../tools/desktop/bspwmrc)); that needs the chosen window manager installed, not GHC or Xmonad.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked --bin compust --example desktop_probe
mkdir -p artifacts
tools/desktop-check.sh artifacts/desktop-present present
tools/desktop-check.sh artifacts/desktop-direct direct
tools/desktop-check.sh artifacts/desktop-effects effects
```

Each report directory must be new. Omitting the mode selects `present`; `--help` describes the command. Set `XVFB` and `XEPHYR` to alternative executable paths to test another server build, including XLibre's Xephyr. They do not select your existing desktop display. `SECONDS_PER_PHASE` accepts 1–30 seconds and defaults to 10. Run measurements sequentially, with other builds and benchmarks stopped. Set `DESKTOP_DISPLAY` and `SERVER_PID` to use an existing dedicated X server instead of starting Xvfb and Xephyr; the runner never stops that server. The [hardware procedure](#run-the-desktop-checks-on-hardware) uses this mode.

The `present` and `direct` configurations use `opacity = 100`, `fade_ms = 0`, `blur_radius = 0`, and `max_fps = 120`. The [Present configuration](../tools/desktop/compust.toml) uses `vsync = true`; the [direct configuration](../tools/desktop/compust-direct.toml) uses `vsync = false`. Those two modes therefore exclude fade, translucency, and blur costs. The [effects configuration](../tools/desktop/compust-effects.toml) uses Present with the default 180 ms fades and blur radius 4. In `effects` mode, the probe also makes the surviving window 50% translucent before measuring, so each redraw blends that window and blurs the full screen behind it.

The probe checks managed tiling, override-redirect popup removal, EWMH fullscreen and restoration, switching to an empty workspace, setting and removing a root wallpaper there, switching back, 32 rapid create/map/destroy sequences, and a surviving window's redraw. Each scene checks actual overlay pixels. With Openbox, windows keep their size inside decorated frames, so the probe also requires a reparenting frame with a painted title bar, overlaps the two windows and raises each in turn, and iconifies and restores one. With i3, a tiling window manager that also reparents, the probe requires the same framed title bars, and first maps a small anchor window that i3 assigns to a second workspace, because i3 keeps only workspaces that are focused or hold a window. With bspwm, a tiling window manager that does not reparent, the runner waits for bspwm's configuration to add the second desktop. bspwm 0.9.12 sometimes keeps the tile of a window that was destroyed right after its map request, so there the last window need not grow to fill the screen, provided the tile left behind shows the background. The runner also requires a successful compositor exit after SIGTERM. Display startup waits for `-displayfd`, while selection, window-manager, and scene readiness use X11 notifications with deadlines.

After two seconds of warmup, the probe measures an idle phase and a phase with a large window, opaque except in `effects` mode, alternating red and blue at a requested 60 updates per second. It records Compust, the X server, the Xvfb host when one is used, Xmonad, and probe CPU separately, with RSS at each phase's endpoints. CPU percentages use one core as 100%; a zero value means no CPU ticks were observed during that interval. Endpoint RSS values are not a long-running leak test.

In `present` and `effects` modes, the active phase must receive multiple compositor Present completions. `frames.csv` contains server UST timestamps, MSC values, serials, and completion modes. Compute intervals only between successive records in the same phase. On nested servers these are software completion timings; on hardware they follow the CRTC's vblank. Neither is input-to-display latency.

In `direct` mode, the probe requires overlay Damage notifications during activity and rejects any compositor Present completions. `frames.csv` has only its header: there is no Present pacing measurement for direct copying. Damage counts establish redraw activity, not displayed frame rate. This probe still requires the server's Present extension so it can detect an incorrectly selected path; production Compust can run without that extension.

## Run the desktop checks on hardware

[`tools/hardware-session.sh`](../tools/hardware-session.sh) runs the three modes on a GPU and physical monitor. It is the client of a new X server started from a text console, so it neither replaces nor disturbs a working session. For example, press Ctrl+Alt+F3, log in, and run:

```sh
cd path/to/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/hardware-desktop -- :20
```

Choose a free display number and an output name from `xrandr`; without `SESSION_OUTPUT`, the first connected output is used. The script stops if the named output is not connected: `xrandr` ignores a name it does not know, and every output would be turned off. The script leaves only that output enabled at its preferred mode, disables screen blanking, and keeps one client connected so the server does not reset between runs. It records the outputs, providers, and GPU, then runs `present`, `direct`, and `effects` against the new server, continuing after a failed mode. `WINDOW_MANAGER` selects the window manager here too. Finally it copies the server log when readable and exits, which ends the session. Do not use the keyboard or mouse until it prints `Hardware session finished`; then log out and return to your usual session. Review `xorg.log` before sharing it: it includes monitor serial numbers and the kernel command line.

## Check shadows, focus, and fullscreen suspension

[`tools/features-check.sh`](../tools/features-check.sh) runs `desktop_probe --features` on an isolated display without a window manager. The [configuration](../tools/desktop/compust-features.toml) disables fades and blur, enables radius-12 shadows at full opacity with offsets of 4 and 6 pixels, enables fullscreen unredirection, and makes the probe's inactive focus windows 50% opaque. Build both release binaries together, then rehearse on Xvfb:

```sh
cargo build --release --locked --bin compust --example desktop_probe
mkdir -p artifacts
SECONDS_PER_PHASE=1 tools/features-check.sh artifacts/features-xvfb xrender
```

Each report directory must be new. This rehearsal starts Xvfb automatically and tests XRender only: Xvfb has no DRI3 and cannot qualify the GPU painter. The default selection, `both`, runs XRender followed by GL on the same dedicated server and compares their shadow and focus images pixel by pixel. It fails if GL never opened the GPU painter or fell back to XRender. `DESKTOP_DISPLAY` and `SERVER_PID` select an existing dedicated server, which the runner never stops; `XVFB` selects another Xvfb executable.

For hardware, log into a text console such as Ctrl+Alt+F3 and run:

```sh
cd ~/compust
env SESSION_OUTPUT=HDMI-1 startx "$PWD/tools/hardware-session.sh" artifacts/features-hardware --features -- :20
```

Choose a connected output and an unused display number as in the desktop procedure above. Omitting `SESSION_OUTPUT` selects the first connected output. No particular window manager is needed. Leave this session in the foreground until `Hardware session finished` appears; it returns to the console, and you can return to the usual desktop.

The probe checks every shadow-scene pixel against an independent box-filter reference, including the window's unchanged interior, all four shadow edges, corners, and the white background. Checks and renderer comparisons allow two levels per RGB channel for rounding. Writing `_NET_ACTIVE_WINDOW` tests both focus directions, `NONE`, and the fallback after removing the property. Scene readiness follows X11 notifications with deadlines.

The same full-screen opaque window alternates red and blue at a requested 60 updates per second in `composed` and `suspended`. A small dialog above it keeps composition active; removing it must unmap the overlay, stop Present completions and overlay Damage, and let drawing reach the root directly. Each state has two seconds of warmup and `SECONDS_PER_PHASE` seconds of measurement (1–30, default 10). Opening another dialog must recapture the content drawn during suspension; removing the full-screen window must restore the background. CPU is recorded separately for Compust, the server, and the probe, without a required saving threshold on short or software runs.

Each painter's report includes `shadows.csv`, the shadow and focus PPM images, `fullscreen-resumed.ppm`, `fullscreen.csv`, `processes.csv`, `frames.csv`, topology and XRes snapshots, the exact configuration, logs, and successful SIGTERM shutdown evidence. GL writes a comparison CSV for each shadow and focus image. The root contains source patches and hashes, binary hashes, base commit and dirty state, CPU/server details, and timestamps. A rehearsal or fallback does not qualify hardware. The [recorded RX 9060 XT session](#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04) covers these fixtures with both painters on one AMD/XLibre display.

## Keep evidence attributable

The report includes server extensions, output geometry, CPU model, kernel, Rust and window-manager versions, the exact configuration, process measurements, scene captures, logs, and executable hashes. `commit.txt` identifies the base revision; `worktree.txt`, `source.patch`, and `source-sha256.txt` record local changes. A dirty run must be described as that revision plus those changes, not as a clean test of the commit.

Retain raw CSV files alongside any summary. Store only captures of isolated test windows in local artifacts; review logs before sharing them. The [earlier records](benchmarks/2026-10-03/) contain the first Present baselines at `1409a919dd0c91ccaad3fbb323df4b33628554fd`. Their archived metadata is less complete than the current runner's report.

## Recorded baseline: 2026-10-03

Four sequential runs passed on CachyOS, Linux 7.2.8-arch1-1, an AMD Ryzen 5 5600GT, Rust 1.95.0, and Xmonad 0.18.1. Each used one 1280×800×24 Xephyr screen hosted by Xorg Xvfb 21.1.24. The nested servers were Xorg 21.1.24 and XLibre 25.1.9. RandR reported a virtual refresh rate of 0.00 Hz; the workload requested 60 updates per second.

The tested base was `cb0796c8e42a95d3c80ab11c557b75809f791d43` plus the [recorded probe/runner changes](benchmarks/2026-10-03/presentation/source.patch) and copied direct-mode configuration. The [source hashes](benchmarks/2026-10-03/presentation/source-sha256.txt) matched across all four runs. The production compositor binary's SHA-256 was `5452d6f590502b1437b8064521d8d82dddd1095d959078fe61ae2d340969a005`, identical to the earlier recorded binary. The source patch excludes the new configuration file; each run archives its actual `compust.toml` separately.

Each active phase lasted approximately 10 seconds and issued 600 updates. CPU values below are percentages of one core; RSS is in KiB. Links lead to the underlying process measurements, with other evidence in the same directory.

| Nested server / path | Active Compust CPU | Active Xephyr CPU | Compust RSS before → after | Present completions / Damage notifications |
| --- | ---: | ---: | ---: | ---: |
| [Xorg / Present](benchmarks/2026-10-03/presentation/xorg-present/processes.csv) | 0.4% | 10.5% | 3,820 → 3,820 | 599 / 599 |
| [Xorg / direct](benchmarks/2026-10-03/presentation/xorg-direct/processes.csv) | 0.3% | 13.3% | 3,876 → 3,876 | 0 / 600 |
| [XLibre / Present](benchmarks/2026-10-03/presentation/xlibre-present/processes.csv) | 0.4% | 13.7% | 3,888 → 3,888 | 599 / 599 |
| [XLibre / direct](benchmarks/2026-10-03/presentation/xlibre-direct/processes.csv) | 0.2% | 9.4% | 3,880 → 3,880 | 0 / 600 |

All measured processes recorded zero idle CPU ticks and unchanged RSS endpoints in both phases. Each Present run observed one completion during idle after warmup; each direct run observed no idle Damage notifications. The Xvfb host used 2.9–4.5% active CPU and the probe 0.3–0.4%, recorded separately. This probe observes Damage in both modes, so the measurements include that observation overhead. These single runs are baselines, not evidence that one server or mode is faster.

| Present intervals, active phase | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| [Xorg, 598 intervals](benchmarks/2026-10-03/presentation/xorg-present/frames.csv) | 16.669 ms | 17.740 ms | 18.728 ms |
| [XLibre, 598 intervals](benchmarks/2026-10-03/presentation/xlibre-present/frames.csv) | 16.669 ms | 17.807 ms | 18.890 ms |

Intervals are successive UST differences divided by 1,000; median and p95 use the nearest-rank convention. All recorded completion modes were `COPY`. Direct-mode frame files contain no timing samples.

Two negative checks swapped only the configurations in temporary runner copies. Expecting Present with a direct compositor [failed for missing completions](benchmarks/2026-10-03/presentation/rejected-present/probe.log); expecting direct copying with a Present compositor [failed for unexpected completions](benchmarks/2026-10-03/presentation/rejected-direct/probe.log). Both exited with status 1, while the four matching runs exited successfully after checking pixels and SIGTERM shutdown.

Xmonad's logs retain `BadAtom` and `getWindowAttributes` diagnostics also present in the earlier archives. Their cause was not resolved in this increment; all asserted scene checks passed. Xephyr's gamma-query warning and virtual refresh rate do not qualify physical display behavior. Hardware hotplug, GPU drivers, decorated/reparenting window managers, and effect-heavy performance remain unverified here.

## Sample monitor transitions

`tools/hotplug-check.sh` runs Compust on an existing X11 session while an operator changes monitors; it does not start a server or window manager. Use a dedicated Xorg or XLibre session. Stop its compositor first, keep the command that restores it, and keep a terminal on a monitor that stays connected. `SERVER_PID` identifies the X server for CPU and RSS measurement; `WM_PID` is optional. Adjust the process names below for your server and window manager.

```sh
cargo build --release --locked --bin compust --example desktop_probe
SERVER_PID=$(pgrep -x Xorg) WM_PID=$(pgrep -n xmonad) \
    tools/hotplug-check.sh artifacts/hotplug-present present
```

After `READY`, make one transition at a time, wait for the desktop to settle, and enter `sample LABEL`; labels use lowercase letters, digits, and hyphens. Enter `quit` when finished. The runner then stops Compust with SIGTERM and requires a successful exit. Each sample does the following:

- writes the root size, each output's connection, CRTC, geometry, mode, and refresh rate, the primary output, and the active RandR monitors to `topology.txt`;
- moves one 128×96 override-redirect marker near opposite corners of every active monitor and across every edge two monitors share, alternating green and red until the overlay shows each color;
- repaints one managed window in place, without moving or resizing it, and requires each color on the overlay; a compositor still showing the window's pixmap from before a resize fails here;
- measures a two-second warmup plus ten-second idle and active phases as in the isolated baseline, writing `processes.csv` and `frames.csv`;
- writes the compositor's XRes resource counts and full owned-pixmap bytes to `resources.csv`.

The probe maps one managed window, which a tiling window manager adds to the current layout, and does not switch workspaces. Keep that window visible and uncovered. Ordinary applications can stay open; their redraws become part of the idle measurement. Pixel checks read the X server framebuffer through the overlay, not light from the panels. The report omits EDID data from `xrandr --verbose` because it contains monitor serial numbers. Local process records still contain command lines, so review the report before sharing it.

A useful sequence starts with a baseline, then a mode change, another layout, and an output disabled, each followed by restoration. For each connector, sample it unplugged while its CRTC is still assigned, after the desktop's reaction (`xrandr --auto` below), reconnected, and restored. Run the sequence once per presentation mode.

## Run the benchmark scenes

`tools/bench.sh` measures Compust's own fixed scenes on an existing X11 session; it starts no server or window manager. Stop the session's compositor first and keep the command that restores it. The [configurations](../tools/bench/) disable fades, use full opacity and `vsync = true`, and select XRender or the optional GL painter, with a 4-pixel radius in the blur variants. `COMPOSITORS` accepts `compust` (the default), `compust-gl`, or both separated by a space. `SCENES` selects a subset. No other compositor is run or required.

```sh
cargo build --release --locked --bin compust --example desktop_probe
SERVER_PID=$(pgrep -x Xorg) WM_PID=$(pgrep -x dwm) tools/bench.sh artifacts/bench
```

Each scene maps a backdrop over the whole root and then override-redirect windows, so no window manager places them and the geometry is the same on any desktop. After a two-second warmup the probe measures for `SECONDS_PER_PHASE`, twenty seconds by default, as in every recorded run. CPU time counts in scheduler ticks of 10 ms, so a 20-second phase resolves 0.05 points of a core.

| Scene | Workload |
| --- | --- |
| `idle` | The backdrop alone |
| `small-update` | A 64×64 window alternating red and blue 60 times per second |
| `fullscreen-translucent` | A 50% window covering the root, alternating the same way |
| `eight-translucent` | Eight overlapping 480×360 windows at 50%; the top one alternates |
| `covered` | The same eight windows under an opaque window covering the root, which alternates |
| `move-resize` | A window moved and resized 60 times per second |
| `open-close` | Windows in turn for the whole phase, each mapped until the overlay shows it, then destroyed until the overlay shows the backdrop; the records through 2026-10-04's occlusion record ran 100 cycles, about 3.3 seconds |

The translucent and covered scenes run without blur and, with a `:blur` suffix, with it. All scenes fit a 1366×768 screen; on a larger one only the full-screen window grows.

Each scene directory holds `summary.csv` with Present intervals, CPU, RSS, open and close latencies, and GPU load; `frames.csv` with every Present completion; `processes.csv`; `latency.csv` for `open-close`; `topology.txt`; `resources.csv`; and the compositor and probe logs. The report root collects the summary rows in its own `summary.csv` and records the commit, Compust configurations, and binary hashes. CPU is a share of one core and, as elsewhere, excludes GPU time. On amdgpu the probe also samples `gpu_busy_percent` ten times a second; `GPU_BUSY` names another load file. An open or close latency runs from the request until the probe reads the change from the overlay after a Present completion or Damage event, so it includes one `GetImage` round trip. `skipped_vblanks` counts vblanks without a completion between consecutive frames, which is meaningful only in scenes that update every vblank. On a multi-monitor root, Present can switch the CRTC it follows, and the new CRTC's MSC has another base; `msc_discontinuities` counts MSC steps that disagree with the time between the frames, whose skipped vblanks are estimated from that time. XRes counts X pixmaps but not GL buffers, so the GL painter's XRes counts alone do not describe its total memory.

Present completes no frames while DPMS has the monitors off. The runner turns them on, disables the screen saver and DPMS for the run, and restores the previous settings afterward. Run it in a dedicated session or on an otherwise idle desktop: other applications' redraws become part of every scene, under either Compust painter.

## Recorded hardware session: 2026-10-03

The session ran on CachyOS with Linux 7.2.8-2-cachyos and native XLibre 25.1.9 using its modesetting driver. It used the amdgpu kernel driver, an AMD Ryzen 5 5600GT with integrated Radeon Vega graphics, Mesa 26.2.4, libdrm 2.4.134, and Xmonad 0.18.1 with xmonad-contrib 0.18.2. The desktop kept its ordinary Xmonad configuration, status bar, and tray. HDMI-1 was the primary output on the right; DP-1, a DisplayPort-to-VGA adapter, was on the left. Both ran 1920×1080 at 60 Hz for a 3840×1080 root. Picom was stopped before the runs and restarted afterward. The [package versions](benchmarks/2026-10-03/hardware/packages.txt) are archived with the reports.

The tested base was `36bd7e8876a6abea564907a610bcabf62794db77` plus the [recorded changes](benchmarks/2026-10-03/hardware/present/source.patch), including the fix below. The [source hashes](benchmarks/2026-10-03/hardware/present/source-sha256.txt) match in both runs, and the compositor binary's SHA-256 was `1220413a892149d2c12bbf56dded047c20bf7956e5063dc246c61a5989ddb2bf`. That hash comes from the combined build command above; building the compositor alone produces a different binary because the probe enables an extra x11rb feature. Both runs used the isolated baseline's [Present](../tools/desktop/compust.toml) and [direct](../tools/desktop/compust-direct.toml) configurations, with fades and blur disabled.

### Stall found and fixed

The first Present run [stopped at its second sample](benchmarks/2026-10-03/hardware/stall/probe.log): after `xrandr --output HDMI-1 --mode 1280x720`, the marker did not redraw within five seconds. A build with [extra logging](benchmarks/2026-10-03/hardware/stall/diagnosis/instrumentation.patch) reproduced it. Its RandR events show a CRTC change at the old 3840×1080 size, a root resize to 3200×1080, and a second CRTC change, all while serial 324 was pending. The server never sent that serial's completion or idle event, and Compust [kept waiting](benchmarks/2026-10-03/hardware/stall/diagnosis/compust-debug.log) instead of rebuilding its buffers. The [roadmap](ROADMAP.md#step-2-on-hardware-physical-hotplug-on-xlibre-with-amd) describes the fix and its regressions.

### Results

Both runs passed all fifteen samples and exited successfully after SIGTERM. Rows correspond to directories `001-baseline` through `015-dp-restored` in the [Present](benchmarks/2026-10-03/hardware/present/) and [direct](benchmarks/2026-10-03/hardware/direct/) reports. CPU is the active-phase percentage of one core; byte counts are identical in both modes.

| Sample | Root | Active monitors | Present: Compust / Xorg CPU | Direct: Compust / Xorg CPU | Owned pixmap bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| Baseline | 3840×1080 | 2 | 0.5% / 4.1% | 0.3% / 3.7% | 49,580,389 |
| HDMI-1 at 1280×720 | 3200×1080 | 2 | 0.4% / 4.2% | 0.4% / 3.8% | 39,544,229 |
| Mode restored | 3840×1080 | 2 | 0.4% / 4.0% | 0.4% / 3.8% | 49,580,389 |
| Vertical layout | 1920×2160 | 2 | 0.4% / 3.9% | 0.3% / 3.8% | 49,763,749 |
| Layout restored | 3840×1080 | 2 | 0.5% / 3.9% | 0.3% / 3.9% | 49,580,389 |
| DP-1 off | 1920×1080 | 1 | 0.4% / 3.4% | 0.3% / 3.3% | 25,000,149 |
| DP-1 on | 3840×1080 | 2 | 0.5% / 4.0% | 0.4% / 3.6% | 49,580,389 |
| HDMI-1 unplugged | 3840×1080 | 2 | 0.4% / 4.1% | 0.4% / 3.9% | 49,580,389 |
| `xrandr --auto` | 1920×1080 | 1 | 0.4% / 3.4% | 0.4% / 3.4% | 25,000,149 |
| HDMI-1 reconnected | 1920×1080 | 1 | 0.4% / 3.4% | 0.3% / 3.7% | 25,000,149 |
| Layout restored | 3840×1080 | 2 | 0.4% / 4.1% | 0.4% / 4.0% | 49,580,389 |
| DP-1 unplugged | 3840×1080 | 2 | 0.4% / 4.2% | 0.5% / 4.4% | 49,580,389 |
| `xrandr --auto` | 1920×1080 | 1 | 0.4% / 3.3% | 0.4% / 3.2% | 25,000,149 |
| DP-1 reconnected | 1920×1080 | 1 | 0.3% / 3.5% | 0.4% / 3.6% | 25,000,149 |
| Layout restored | 3840×1080 | 2 | 0.3% / 4.1% | 0.4% / 3.8% | 49,580,389 |

Each active phase issued 600 marker updates. Present samples received 600 or 601 completions, all `COPY`. Direct samples received no completions and 709–728 overlay Damage notifications, including redraws caused by other clients. During idle phases, Compust used 0.0–0.2% and Xorg 0.9–1.5% of one core; the desktop's own clients still caused 126–147 compositor redraws per ten seconds.

Across 8,988 active-phase Present intervals, the median and nearest-rank p95 were 16.667 ms and the maximum was 16.670 ms. Samples in which DP-1 covered most of the root, with HDMI-1 at 1280×720 or disabled, had a 16.635 ms median, consistent with Present following DP-1's CRTC; the others had 16.667 ms. While HDMI-1 was unplugged but still assigned a CRTC, its intervals continued at 16.667 ms.

Every repeated topology reproduced the same `resources.csv` in each mode. With one monitor active, the compositor held one fewer window pixmap, picture, and Damage object because one fewer window was mapped; direct mode has no Present event selection. Compust RSS rose from 3,884 to 3,896 KiB in the Present run, unchanged from the third sample onward, and from 3,760 to 3,772 KiB in the direct run, unchanged from the sixth. Xorg RSS rose from 118,604 to 119,244 KiB during the first mode change and reached 119,340 KiB by the end of the direct run.

These results qualify monitor transitions for this environment only. They do not cover other GPU drivers, Xorg on hardware, mixed refresh rates, more than two monitors, fades, blur, or long-running sessions. Turning DP-1 off with `xrandr` also moved HDMI-1 to the origin and shrank the root. CRTC changes without a root resize are covered by the Xvfb regression instead. DP-1's hotplug is the adapter's DisplayPort connection. The marker checks prove server-side redraws, not what each panel displayed.

## Recorded effects baseline: 2026-10-03

Six sequential runs repeated the isolated baseline in all three modes with Xorg Xephyr 21.1.24 and XLibre Xephyr 25.1.9. The base was `0ef4d14f0e5e81fff8cf3689f008961e3c1ed15e` plus the [recorded changes](benchmarks/2026-10-03/effects/xorg-present/source.patch), with the compositor binary `1220413a892149d2c12bbf56dded047c20bf7956e5063dc246c61a5989ddb2bf`. Every run passed every scenario, including the wallpaper change.

| Nested server / mode | Active Compust CPU | Active Xephyr CPU | Frames in 10 s | Interval median / p95 |
| --- | ---: | ---: | ---: | ---: |
| [Xorg / Present](benchmarks/2026-10-03/effects/xorg-present/processes.csv) | 0.4% | 10.4% | 599 | 16.679 / 17.676 ms |
| [Xorg / direct](benchmarks/2026-10-03/effects/xorg-direct/processes.csv) | 0.3% | 10.4% | 600 Damage | — |
| [Xorg / effects](benchmarks/2026-10-03/effects/xorg-effects/processes.csv) | 0.0% | 85.8% | 85 | 116.681 / 118.518 ms |
| [XLibre / Present](benchmarks/2026-10-03/effects/xlibre-present/processes.csv) | 0.6% | 14.3% | 598 | 16.673 / 17.711 ms |
| [XLibre / direct](benchmarks/2026-10-03/effects/xlibre-direct/processes.csv) | 0.3% | 9.9% | 600 Damage | — |
| [XLibre / effects](benchmarks/2026-10-03/effects/xlibre-effects/processes.csv) | 0.0% | 86.1% | 85 | 116.640 / 118.027 ms |

Present and direct results match the earlier baseline within run-to-run variation. For direct runs, frames are overlay Damage notifications. With the translucent survivor and blur radius 4, Xephyr spent most of a core on 1280×800 frames and delivered about 8.5 per second, while Compust recorded no CPU ticks.

## Recorded hardware desktop session: 2026-10-03

The [AMD/XLibre machine above](#recorded-hardware-session-2026-10-03) ran a dedicated XLibre 25.1.9 server on vt3, started by `startx` from a text console while the usual session stayed on vt2. The [server log](benchmarks/2026-10-03/desktop-hardware/xorg.log) reports glamor on radeonsi with OpenGL 4.6, and TearFree enabled by the modesetting driver's default. HDMI-1 was the only active output, at 1920×1080 and 60 Hz; DP-1 was off. The tested tree and binaries match the nested effects baseline. All three modes passed every scenario; the [session log](benchmarks/2026-10-03/desktop-hardware/session.log) lists the results.

| Mode | Active Compust CPU | Active Xorg CPU | Frames in 10 s | Interval median / p95 / max |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/desktop-hardware/present/processes.csv) | 0.3% | 3.1% | 600 | 16.667 / 16.667 / 16.667 ms |
| [Direct](benchmarks/2026-10-03/desktop-hardware/direct/processes.csv) | 0.3% | 3.5% | 600 Damage | — |
| [Effects](benchmarks/2026-10-03/desktop-hardware/effects/processes.csv) | 0.0% | 93.7% | 49 | 199.998 / 216.665 / 216.665 ms |

Present followed vblank exactly: every active-phase MSC advanced by one. In idle phases, Compust and Xorg recorded no CPU ticks in Present and direct modes; in effects mode, Xorg used 1.9% of a core finishing the last warmup frame. Compust RSS stayed at 3,824–3,888 KiB and Xorg RSS at 92,820–93,152 KiB, unchanged within each phase.

With blur, each frame took 12 or 13 vblanks while Xorg used most of a CPU core, which is slower per frame than software Xephyr at 1280×800. Glamor accelerates only nearest and bilinear filtering; [`glamor_composite`](https://github.com/X11Libre/xserver/blob/b4b92c2374ec81ea979d53ad79fd0d0784bbf291/glamor/glamor_render.c#L1766-L1769) sends any convolution filter to its software fallback, which moves pixmaps between GPU and CPU memory. Compust's two convolution passes therefore run on the CPU for every redraw of a translucent window. On this machine, a full-screen translucent window with blur radius 4 limits output to about five frames per second.

These runs use the probe's synthetic windows, one monitor, Xmonad, and ten-second phases. They do not cover actual applications, decorated or reparenting window managers, server shutdown under a running compositor, mixed refresh rates, or other GPUs. The [pyramid blur session](#recorded-pyramid-blur-session-2026-10-03) repeats these runs after removing the convolution.

## Recorded pyramid blur session: 2026-10-03

The convolution passes were then replaced by a bilinear pyramid, described in the [roadmap](ROADMAP.md#blur-on-the-gpu-path). The same runs were repeated with compositor binary `6826205798e183b039d558a54794e506732bdc88560d8f6ae6a13eaa05baa35f`: six [nested runs](benchmarks/2026-10-03/pyramid/) and a [dedicated hardware session](benchmarks/2026-10-03/desktop-hardware-pyramid/) under the conditions above. All passed every scenario.

| Effects mode | Frames in 10 s | X server CPU | Interval median |
| --- | ---: | ---: | ---: |
| Xorg Xephyr, 1280×800: convolution → pyramid | 85 → 598 | 85.8% → 20.4% | 116.681 → 16.699 ms |
| XLibre Xephyr, 1280×800: convolution → pyramid | 85 → 599 | 86.1% → 19.4% | 116.640 → 16.655 ms |
| AMD/XLibre hardware, 1920×1080: convolution → pyramid | 49 → 599 | 93.7% → 4.1% | 199.998 → 16.667 ms |

On hardware, every active-phase MSC advanced by one, and Compust used 0.4% of a core. Blur raised Xorg from 3.1% in the Present run to 4.1%. Present and direct results stayed within the run-to-run variation of the earlier records. The pyramid looks smoother and slightly stronger than the box filter at radius 4.

## Recorded Intel/Xorg sessions: 2026-10-03

A second machine ran Linux Mint 22.3 with Linux 7.0.0-34-generic and native Xorg 21.1.11, using the modesetting driver with glamor. It has an Intel Core i3-1005G1 with integrated Iris Plus G1 graphics (Ice Lake, i915 kernel driver, Mesa 25.2.8) and one 1366×768 panel, eDP-1, at 60.06 Hz. Xmonad was 0.17.2 with xmonad-contrib 0.17.1. The [records](benchmarks/2026-10-03/intel-xorg/) omit scene captures, and the server log has its hostname and kernel command line removed.

The nested and monitor runs used the clean commit `ffd0b13222fa63723e8d1dac649fb1e0388e040e` with compositor binary `28c8a0622cda431433f77f60571c4880609766d7d857bc4143d4e259f94e676c`. The dedicated session used that commit plus the [recorded Present timeout change](benchmarks/2026-10-03/intel-xorg/desktop/present/source.patch), with binary `6b6c51f0e44aabb601c6fbd24bb1d43e341b8875e3f932f24fc5a2eb33f2ff0c`.

### Nested baseline

Xorg Xephyr 21.1.11 hosted by Xvfb at 1280×800 passed every scenario in all three modes.

| Mode | Active Compust CPU | Active Xephyr CPU | Frames in 10 s | Interval median / p95 |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/intel-xorg/nested/present/processes.csv) | 0.8% | 18.0% | 599 | 16.680 / 17.753 ms |
| [Direct](benchmarks/2026-10-03/intel-xorg/nested/direct/processes.csv) | 0.3% | 17.7% | 600 Damage | — |
| [Effects](benchmarks/2026-10-03/intel-xorg/nested/effects/processes.csv) | 0.9% | 32.4% | 594 | 16.749 / 17.846 ms |

### Monitor transitions on the panel

The ordinary Xmonad desktop stayed open with its status bar, tray, terminal, and browser. The [monitor-transition runner](#sample-monitor-transitions) sampled the baseline, eDP-1 at 1280×720, and the restored mode, once per presentation mode. All six samples passed, and Compust exited successfully after SIGTERM. CPU is the active-phase percentage of one core.

| Sample | Root | Present: Compust / Xorg CPU | Direct: Compust / Xorg CPU | Owned pixmap bytes |
| --- | --- | ---: | ---: | ---: |
| Baseline | 1366×768 | 0.8% / 6.3% | 0.8% / 4.2% | 8,364,965 |
| eDP-1 at 1280×720 | 1280×720 | 0.8% / 6.4% | 0.7% / 4.1% | 7,358,677 |
| Mode restored | 1366×768 | 0.9% / 5.5% | 0.8% / 4.2% | 8,364,965 |

Each [Present sample](benchmarks/2026-10-03/intel-xorg/monitors/present/) received 601 completions, all `COPY`. Across its 1,800 active-phase intervals the median was 16.650 ms and the maximum 16.671 ms, and every MSC advanced by one. [Direct samples](benchmarks/2026-10-03/intel-xorg/monitors/direct/) received 707–711 overlay Damage notifications. During idle phases, Compust used 0.2–0.3% and Xorg 0.9–1.4% of one core while the desktop's own clients caused 116–134 redraws per ten seconds. The baseline and restored samples reported identical `resources.csv` in each mode. Compust RSS stayed within 3,416–3,668 KiB, and Xorg RSS went from 63,916 to 64,468 KiB over both runs.

### Dedicated desktop session

[`hardware-session.sh`](../tools/hardware-session.sh) ran on a new server on vt3 while the usual session stayed on vt7. All three modes passed every scenario; the [session log](benchmarks/2026-10-03/intel-xorg/desktop/session.log) lists the results.

| Mode | Active Compust CPU | Active Xorg CPU | Frames in 10 s | Interval median / p95 / max |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-03/intel-xorg/desktop/present/processes.csv) | 0.7% | 4.3% | 600 | 16.650 / 16.664 / 33.294 ms |
| [Direct](benchmarks/2026-10-03/intel-xorg/desktop/direct/processes.csv) | 0.5% | 3.0% | 600 Damage | — |
| [Effects](benchmarks/2026-10-03/intel-xorg/desktop/effects/processes.csv) | 0.7% | 4.4% | 600 | 16.651 / 16.664 / 33.294 ms |

The Present and effects runs each had one interval spanning two vblanks; in the other 598, the MSC advanced by one. In idle phases, Compust and Xorg recorded no CPU ticks. Compust RSS stayed at 3,580–3,664 KiB and Xorg RSS at 99,804–99,812 KiB, unchanged within each phase. Blur behind the full-screen translucent window raised Xorg from 4.3% to 4.4% of a core, so the pyramid also stays on the GPU with glamor on this Intel driver.

Two informal checks have no archived record. The usual session's compositor, running Present with default fades and blur, kept updating the status bar clock after an eight-second forced DPMS-off, and again after the switch to vt3 and back; it logged no Present timeout.

These sessions use the probe's synthetic windows, one panel, Xmonad 0.17.2, and ten-second phases. The laptop has a single display, so physical unplugging and reconnection were not tested in these sessions; a [later session](#recorded-intelxorg-hotplug-session-2026-10-03) adds an external display. Actual applications, other window managers, and mixed refresh rates remain open; a [later record](#recorded-suspend-and-resume-2026-10-03) covers suspend and resume.

## Recorded Intel/Xorg hotplug session: 2026-10-03

The [Intel/Xorg laptop](#recorded-intelxorg-sessions-2026-10-03) ran the [monitor-transition procedure](#sample-monitor-transitions) with an external 1920×1080 display on HDMI-1, to the right of the 1366×768 panel, in an ordinary i3 4.23 desktop. The panel refreshes at 60.059 Hz and the external display at 60.000 Hz, so this is also the first record with different resolutions and slightly different refresh rates. The tree was the clean commit `0485ddcdec633859d3decca695238e2e13e411da` with compositor binary `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`, and fades and blur disabled.

Both presentation modes passed eleven samples and exited successfully after SIGTERM: the baseline, a 1280×720 mode on HDMI-1 and its restoration, a vertical layout and its restoration, HDMI-1 off and on, then physical unplugging with the CRTC still assigned, `xrandr --auto`, reconnection, and restoration. CPU is the active-phase percentage of one core.

| Sample | Root | Active monitors | Present: Compust / Xorg CPU | Present interval median | Direct: Compust / Xorg CPU | Owned pixmap bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 3286×1080 | 2 | 1.2% / 6.5% | 16.677 ms | 1.0% / 4.4% | 18,617,665 |
| HDMI-1 at 1280×720 | 2646×768 | 2 | 1.1% / 5.7% | 16.656 ms | 1.2% / 4.6% | 12,491,777 |
| Mode restored | 3286×1080 | 2 | 1.6% / 6.8% | 16.670 ms | 0.7% / 4.2% | 18,617,665 |
| Vertical layout | 1920×1848 | 2 | 1.4% / 6.8% | 16.668 ms | 0.6% / 4.2% | 18,614,785 |
| Layout restored | 3286×1080 | 2 | 1.5% / 6.5% | 16.668 ms | 0.6% / 4.1% | 18,617,665 |
| HDMI-1 off | 1366×768 | 1 | 1.2% / 6.0% | 16.632 ms | 0.7% / 4.2% | 8,441,857 |
| HDMI-1 on | 3286×1080 | 2 | 1.1% / 6.2% | 16.657 ms | 1.2% / 4.8% | 18,617,665 |
| HDMI-1 unplugged | 3286×1080 | 2 | 1.2% / 5.7% | 16.662 ms | 1.1% / 4.6% | 18,617,665 |
| `xrandr --auto` | 1366×768 | 1 | 1.5% / 6.3% | 16.647 ms | 1.2% / 4.5% | 8,441,857 |
| HDMI-1 reconnected | 1366×768 | 1 | 1.0% / 6.1% | 16.654 ms | 0.9% / 4.3% | 8,441,857 |
| Layout restored | 3286×1080 | 2 | 1.1% / 6.7% | 16.669 ms | 1.0% / 4.6% | 18,617,665 |

Each sample checks marker redraws near opposite corners of every active monitor and across the edge the two monitors share. Every topology that occurs more than once reproduced the same `resources.csv` in each mode, including the unplugged sample against the baseline. Compust logged no Present timeout. Its RSS went from 3,432 to 3,476 KiB over the [Present run](benchmarks/2026-10-03/intel-xorg/hotplug/present/) and from 3,400 to 3,496 KiB over the [direct run](benchmarks/2026-10-03/intel-xorg/hotplug/direct/); Xorg RSS stayed at 96,616 KiB throughout.

Across 6,590 active-phase Present intervals, the median was 16.662 ms and the nearest-rank p95 16.679 ms. Three samples each had one interval spanning two vblanks; in the rest, every MSC advanced by one. The median was 16.632–16.654 ms with the panel alone and 16.656–16.677 ms with both outputs, consistent with Present following one CRTC rather than each monitor's own refresh. In idle phases Compust used at most 0.1% of a core.

This session did not detect a defect that the same cable test showed in ordinary use: tiled windows kept stale contents after i3 resized and restored them. At the time, the runner checked only its own override-redirect marker, which a window manager does not resize; it now also repaints a managed window, and with that check the 0.2.0-beta.2 binary fails the sample after HDMI-1 is turned off. The [roadmap](ROADMAP.md#local-beta-testing-stale-window-after-a-restored-resize) describes the defect and its fix, which came after these records.

One connector on one external display was tested, with Xorg and i3. The marker checks prove server-side redraws, not what each screen displayed. Refresh rates that differ by more than this, more than two monitors, and fades or blur during a transition remain untested.

## Recorded Openbox sessions: 2026-10-03

The [Intel/Xorg laptop above](#recorded-intelxorg-sessions-2026-10-03) repeated the three procedures with Openbox 3.6.1, a stacking window manager that reparents each client into a decorated frame. The base was `bbfbd68489a5550bd9d3aa70c587244097d33028`, the 0.2.0-beta.2 release commit, plus the [recorded probe and runner changes](benchmarks/2026-10-03/intel-xorg/openbox/desktop/present/source.patch); the compositor source is unchanged, and its binary was `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. The [records](benchmarks/2026-10-03/intel-xorg/openbox/) follow the same conventions as the Xmonad ones.

Every run passed the existing scenarios and the three added for stacking window managers: decorated frames, restacking of overlapping windows, and iconify and restore.

| Run | Active Compust CPU | Active X server CPU | Frames in 10 s | Interval median / p95 / max |
| --- | ---: | ---: | ---: | ---: |
| [Nested, Present](benchmarks/2026-10-03/intel-xorg/openbox/nested/present/processes.csv) | 0.8% | 13.4% | 599 | 16.690 / 17.804 / 19.867 ms |
| [Nested, direct](benchmarks/2026-10-03/intel-xorg/openbox/nested/direct/processes.csv) | 0.3% | 14.7% | 600 Damage | — |
| [Nested, effects](benchmarks/2026-10-03/intel-xorg/openbox/nested/effects/processes.csv) | 1.1% | 19.6% | 587 | 16.681 / 17.890 / 33.907 ms |
| [Hardware, Present](benchmarks/2026-10-03/intel-xorg/openbox/desktop/present/processes.csv) | 0.8% | 4.6% | 599 | 16.650 / 16.663 / 33.301 ms |
| [Hardware, direct](benchmarks/2026-10-03/intel-xorg/openbox/desktop/direct/processes.csv) | 0.6% | 2.9% | 600 Damage | — |
| [Hardware, effects](benchmarks/2026-10-03/intel-xorg/openbox/desktop/effects/processes.csv) | 0.7% | 4.7% | 600 | 16.650 / 16.663 / 33.291 ms |

The nested server is Xorg Xephyr 21.1.11 at 1280×800; the hardware rows come from a dedicated session on vt3 at 1366×768. As with Xmonad, each hardware Present run had one interval spanning two vblanks. In idle phases on hardware, Compust and Xorg recorded no CPU ticks, and RSS was unchanged within each phase: 3,380–3,440 KiB for Compust.

The [monitor-transition runner](benchmarks/2026-10-03/intel-xorg/openbox/monitors/) also passed the baseline, eDP-1 at 1280×720, and the restored mode in both presentation modes on an ordinary Openbox desktop. Present intervals had a 16.65 ms median and a 16.67 ms maximum, and every MSC advanced by one. A terminal was animating during those samples, causing 153–550 idle redraws per ten seconds, so their CPU figures are not comparable with the Xmonad samples. The direct run's baseline and restored samples reported identical `resources.csv`; the Present run's did not, because the number of open desktop windows changed between samples.

One Openbox behavior affected the probe. When a client mapped its window at the moment Compust claimed the screen, Openbox 3.6.1 left the map request unhandled until its next event; a later root property change released it. The probe now sends such property changes during its first wait under a stacking window manager. The cause inside Openbox was not investigated.

These sessions use the probe's synthetic windows. Interactive moving and resizing, Openbox's own menus, and actual applications have no recorded scenario.

## Recorded i3 sessions: 2026-10-03

The same laptop repeated the three procedures with i3 4.23, a tiling window manager that reparents each client into a frame with a title bar. The base was `025cabdd0cd690a960f1891613c9a28186a5cfdf` plus the [recorded probe and runner changes](benchmarks/2026-10-03/intel-xorg/i3/desktop/present/source.patch); the compositor source is unchanged, and its binary was `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. Every run passed every scenario, including the framed title bars and the switch to the anchor's workspace.

| Run | Active Compust CPU | Active X server CPU | Frames in 10 s | Interval median / p95 / max |
| --- | ---: | ---: | ---: | ---: |
| [Nested, Present](benchmarks/2026-10-03/intel-xorg/i3/nested/present/processes.csv) | 0.8% | 18.3% | 599 | 16.679 / 17.776 / 18.984 ms |
| [Nested, direct](benchmarks/2026-10-03/intel-xorg/i3/nested/direct/processes.csv) | 0.3% | 17.9% | 600 Damage | — |
| [Nested, effects](benchmarks/2026-10-03/intel-xorg/i3/nested/effects/processes.csv) | 0.8% | 32.7% | 584 | 16.683 / 17.871 / 34.548 ms |
| [Hardware, Present](benchmarks/2026-10-03/intel-xorg/i3/desktop/present/processes.csv) | 0.7% | 4.4% | 599 | 16.650 / 16.664 / 33.296 ms |
| [Hardware, direct](benchmarks/2026-10-03/intel-xorg/i3/desktop/direct/processes.csv) | 0.4% | 2.9% | 600 Damage | — |
| [Hardware, effects](benchmarks/2026-10-03/intel-xorg/i3/desktop/effects/processes.csv) | 0.9% | 4.6% | 600 | 16.650 / 16.664 / 33.318 ms |

The nested server is Xorg Xephyr 21.1.11 at 1280×800; the hardware rows come from a dedicated session on vt3 at 1366×768. Each hardware Present run had one interval spanning two vblanks. In idle phases on hardware, Compust and Xorg recorded no CPU ticks, and Compust RSS stayed at 3,392–3,412 KiB.

The [monitor-transition runner](benchmarks/2026-10-03/intel-xorg/i3/monitors/) passed the baseline, eDP-1 at 1280×720, and the restored mode in both presentation modes on an ordinary i3 desktop with its bar. Active-phase CPU was 0.6–0.8% for Compust and 5.7–6.5% for Xorg with Present, and 0.6–0.7% and 3.9–4.2% with direct copying. Present intervals had a 16.65 ms median; two of the three samples had one interval spanning two vblanks. The baseline and restored samples reported identical `resources.csv` in each mode.

These sessions use the probe's synthetic windows in i3's default split layout. Its stacked and tabbed layouts, floating windows other than the anchor, and actual applications have no recorded scenario.

## Recorded suspend and resume: 2026-10-03

The [monitor-transition runner](#sample-monitor-transitions) also samples around a suspend: take one sample, run `systemctl suspend`, wake the machine, and sample again. On the [Intel/Xorg laptop](#recorded-intelxorg-sessions-2026-10-03), in an ordinary Openbox 3.6.1 desktop, each presentation mode took one sample before suspending to RAM (`deep` in `/sys/power/mem_sleep`), one about six seconds after waking, and one more after the second had finished. The tree was the clean commit `6f8d505ab55bd3db59605ca33a7b4fce95df6bdc` with compositor binary `c3d810af11586206eff8c893d395a3b4a7c3f93bfcf43d71f27f8c7946d7bf31`. The wall clock ran 197 seconds ahead of the monotonic clock across the Present suspension and 54 seconds across the direct one; the kernel log was not readable to confirm the sleep state reached.

| Sample | Present: Compust / Xorg CPU | Present completions, intervals over one vblank | Direct: Compust / Xorg CPU | Owned pixmap bytes |
| --- | ---: | ---: | ---: | ---: |
| Before suspend | 1.2% / 8.7% | 600, 1 | 0.5% / 3.4% | 8,459,037 |
| After resume | 0.8% / 6.2% | 600, 0 | 0.8% / 4.4% | 8,459,037 |
| After resume, settled | 0.9% / 8.5% | 598, 3 | 0.7% / 4.3% | 8,459,037 |

All six samples in the [Present](benchmarks/2026-10-03/intel-xorg/suspend/present/) and [direct](benchmarks/2026-10-03/intel-xorg/suspend/direct/) reports passed their marker redraw checks, and Compust exited successfully after SIGTERM in both runs. Present intervals had a 16.65 ms median in every sample. Compust logged no Present timeout, so completion events resumed on their own here. Its RSS went from 3,248 to 3,336 KiB over the Present run and stayed at 3,344 KiB in the direct run.

This is one suspension per mode on one machine, with fades and blur disabled. It does not cover hibernation, suspending with a pending animation, or other drivers.

## Recorded reload session: 2026-10-03

A third machine kept one Compust process through three [monitor-sampler](#sample-monitor-transitions) runs and reloaded its configuration with SIGUSR1 between samples. It has an AMD Ryzen 5 5600X and a Radeon RX 9060 XT (Navi 44, amdgpu kernel driver, radeonsi in Mesa 26.2.4), and runs CachyOS with Linux 7.2.8 and native XLibre 25.1.9 with its built-in modesetting driver. dwm 6.8 managed an ordinary desktop on DP-2 at 1920×1080 and 60 Hz; HDMI-1, also 1920×1080 at 60 Hz, was turned on beside it for the two-monitor samples. The base was `4d7b058b124dbac47df68f05424930891bda65ea` plus the [recorded patch](benchmarks/2026-10-03/reload-amd-dwm/source.patch) that adds reload, with compositor binary `87ab455db7f64f0c7fe20d0d2e836ea9a45c8676b2c83dca4b10dcfb0471f2b7`. The [runner](benchmarks/2026-10-03/reload-amd-dwm/runner.sh) follows `tools/hotplug-check.sh` but keeps the compositor running between probe runs, with three-second phases.

| Step before the sample | Root | Running settings | Present interval median | Owned pixmap bytes |
| --- | --- | --- | ---: | ---: |
| Start, Present run | 1920×1080 | Present, no fade or blur | 16.667 ms | 16,637,953 |
| HDMI-1 on, right of DP-2 | 3840×1080 | same | 16.667 ms | 25,093,633 |
| Reload `fade_ms = 120`, `blur_radius = 4` | 3840×1080 | blur 4 | 16.667 ms | 30,277,633 |
| Reload with `opacity = 101`, rejected | 3840×1080 | unchanged | 16.667 ms | 30,277,633 |
| HDMI-1 left of DP-2 | 3840×1080 | unchanged | 16.667 ms | 30,277,633 |
| Reload `blur_radius = 16` | 3840×1080 | blur 16 | 16.667 ms | 30,602,113 |
| Reload without fade or blur; HDMI-1 right again | 3840×1080 | no fade or blur | 16.667 ms | 25,093,633 |
| Reload `vsync = false`; direct probe run | 3840×1080 | direct | — | 25,093,633 |
| Reload `blur_radius = 4` | 3840×1080 | direct, blur 4 | — | 30,277,633 |
| HDMI-1 off | 1920×1080 | direct, blur 4 | — | 19,229,953 |
| Reload `vsync = true`, no blur; Present probe run | 1920×1080 | Present, no blur | 16.667 ms | 16,637,953 |
| HDMI-1 on, right of DP-2 | 3840×1080 | same | 16.667 ms | 25,093,633 |

All twelve samples passed the marker checks on each monitor and across the shared edge, and the repaint of the window dwm tiled. The probe also checks the presentation path: a direct run fails on any Present completion and a Present run requires them, so the `vsync` reloads switched the path on hardware in both directions. Each combination of topology and settings that occurs more than once reproduced the same owned pixmap bytes after the renderer had been replaced in between. The rejected reload logged a warning and left the running settings in place. Across 1,611 active-phase Present intervals the median was 16.667 ms, the nearest-rank p95 16.674 ms, and the maximum 16.678 ms; every MSC advanced by one. In active phases Compust used at most 0.7% of a core and the X server 7.7–8.0%, with the desktop's own applications running; Compust RSS went from 3,920 to 3,936 KiB. Compust exited successfully after SIGTERM, and the output configuration ended as it began.

The windows are the probe's synthetic ones. No window opened or closed while fades were enabled, so no fade crossed a reload; `opacity` and `max_fps` reloads and fades in progress are covered only by the [Xvfb tests](../tests/cases/reload.rs). HDMI-1 was switched with `xrandr`, not unplugged, and the server log was not readable to confirm glamor acceleration.

## Recorded benchmark scenes: 2026-10-03

Compust ran twice with both monitors and twice with DP-2 alone on the [RX 9060 XT desktop](#recorded-reload-session-2026-10-03), with 20-second phases at the clean commit `394b7e63404e8eac5028e7dc933f7458dd7fa696`. No application redrew during the runs. The [raw records](benchmarks/2026-10-03/bench-amd-dwm/) preserve the original measurements; the tables below report Compust's workloads.

Each cell is Compust's CPU plus the X server's, as a percentage of one core and the mean of the two runs; the runs agree within 0.25 points. With nothing changing on screen the server used 5.4%, which is this machine's baseline. Active scenes held 60 frames per second without a skipped vblank; the idle scene presented no frames.

Two monitors, 3840×1080:

| Scene | Compust  |
| --- | ---:  |
| Idle | 0.0 + 5.4%  |
| Small window updating | 0.3 + 7.8%  |
| Full-screen translucent | 0.3 + 8.0%  |
| Full-screen translucent, blur | 0.4 + 8.2%  |
| Eight translucent | 0.4 + 8.5%  |
| Eight translucent, blur | 1.0 + 9.4%  |
| Move and resize | 1.1 + 8.8%  |
| Open and close | 0.4 + 9.1%  |

DP-2 alone, 1920×1080:

| Scene | Compust  |
| --- | ---:  |
| Idle | 0.0 + 5.4%  |
| Small window updating | 0.2 + 7.4%  |
| Full-screen translucent | 0.2 + 7.3%  |
| Full-screen translucent, blur | 0.4 + 7.6%  |
| Eight translucent | 0.4 + 7.9%  |
| Eight translucent, blur | 1.0 + 8.8%  |
| Move and resize | 1.1 + 8.4%  |
| Open and close | 0.4 + 8.6%  |

Compust's own process used 0.2–1.1% of a core and 3,788–3,936 KiB of RSS. The small-window scene reported 8.4% GPU load on two monitors; this build repainted and copied the whole 3840×1080 frame for each change. The GPU changes its clock with load, so `gpu_busy_percent` is only an approximate measure of work. In the idle two-monitor scene, XRes reported 41.6 MB of pixmaps owned by Compust; XRes does not include GL buffers.

In each run, 46–58 of the 100 new windows appeared one frame after the map request (16.3–16.7 ms), and the rest after two (about 33.2 ms). This split explains the variation in median open latency; `latency.csv` lists every cycle. Closed windows disappeared one frame after the request, within 17.5 ms. Later [repaint changes](#recorded-repaint-change-2026-10-03) removed the redundant frame.

This record covers one machine with a fast discrete GPU and 60 Hz monitors, synthetic windows, and a backdrop covering the desktop. It does not cover a 4K screen, a slow GPU, Xorg, or the Intel/Xorg laptop. Latencies include the probe's overlay read and do not measure the panels.

## Recorded event-path change: 2026-10-03

After [step 2 of the rendering milestone](ROADMAP.md#2-event-path-round-trips), the benchmark runner repeated the small-window, move-and-resize, and open-and-close scenes for Compust on the same desktop, twice with both monitors and twice with DP-2 alone, at the clean commit `5f088b6cf48041815ac0a830b5209e9fa87c1407`. The [records](benchmarks/2026-10-03/event-path-amd-dwm/) follow the [earlier ones](#recorded-benchmark-scenes-2026-10-03), which are the "before" column. Each cell is Compust's CPU plus the X server's, as a mean of two runs.

| Scene | Monitors | Before | After |
| --- | --- | ---: | ---: |
| Move and resize | Two | 1.08 + 8.78% | 0.95 + 8.65% |
| Move and resize | One | 1.05 + 8.35% | 0.93 + 8.20% |
| Small window updating | Two | 0.30 + 7.75% | 0.28 + 8.05% |
| Small window updating | One | 0.25 + 7.40% | 0.28 + 7.38% |
| Open and close | Two | 0.36 + 9.09% | 0.25 + 9.22% |
| Open and close | One | 0.37 + 8.64% | 0.24 + 8.64% |

Moving and resizing cost Compust 0.13 points less and the server 0.13–0.15 points less in both layouts. The scene resizes the window every frame, so each event still recaptures it, which takes about ten round trips; the change removes only the tree, geometry, and shape queries. A pure move now costs none, as the [regression](../tests/cases/event_path.rs) checks. The other differences are within the variation between runs.

In the first two-monitor run, the small-window scene missed 17 vblanks in less than half a second. Present's MSC jumped by 2^24 + 5 there and then advanced by 4 over eight vblanks' time, consistent with Present switching between the two monitors' CRTCs, whose counters have different bases. The overlay covers both monitors equally, and the primary output, DP-1, is disconnected. Compust logged no Present timeout, and that run's server figure, 8.25%, raises the two-monitor mean above. No other scene in these records or the earlier ones shows a discontinuity. The probe now [counts them separately](#run-the-benchmark-scenes); this run's `summary.csv`, written before that change, reports the raw MSC step as 16,777,229 skipped vblanks.

## Recorded repaint change: 2026-10-03

The [benchmark records](#recorded-benchmark-scenes-2026-10-03) showed about half of new windows one frame later under Compust than the rest. A temporarily instrumented build logged each event and Present submission during the open-and-close scene. In every such cycle, the window's `UnmapNotify` and `DestroyNotify` were read in separate batches: the unmap alone removed the window and its frame was submitted, then the destroy requested another, identical frame. That frame went out as soon as the first completed, and the next window, mapped right after, waited a vblank for it because the single Present buffer is reused only after each submission finishes.

Compust now repaints only for events that change a shown surface or their order. At the clean commit `338bb0777e2fe1ab87986ea71d130393a7ce70b2`, the same three scenes ran twice per monitor layout. Every one of the 400 new windows appeared one frame after its map request, in at most 17.3 ms, against 46–58 per 100 before; closes still took one frame. The 100 cycles took 3.33 seconds instead of 4.1–4.3, with exactly two Present completions each. CPU in the small-window and move-and-resize scenes stayed within the variation between runs, and no scene skipped a vblank. The [records](benchmarks/2026-10-03/repaint-amd-dwm/) follow the earlier ones.

## Recorded region repaint: 2026-10-04

After [step 3 of the rendering milestone](ROADMAP.md#3-region-based-repaint), the runner measured every scene on the same desktop with the previous commit `da928ad9f424736f28caf4a3317bcdf40de82a48` and the region-repaint commit `a7efd319870ee808252bd649ebcdbac12007a4d3`, alternating the two builds, twice with both monitors and twice with DP-2 alone. Both builds had clean sources and used the same probe binary. Each CPU cell is Compust's CPU plus the X server's, as a mean of two runs; GPU is the mean amdgpu busy percentage. The [records](benchmarks/2026-10-04/region-amd-dwm/) include the [sequence](benchmarks/2026-10-04/region-amd-dwm/sequence.sh) that ran them.

| Scene | Monitors | Before | After | GPU before | GPU after |
| --- | --- | ---: | ---: | ---: | ---: |
| Idle | Two | 0.00 + 5.97% | 0.00 + 5.35% | 0.1% | 0.0% |
| Idle | One | 0.00 + 5.35% | 0.00 + 5.35% | 0.0% | 0.0% |
| Small window updating | Two | 0.30 + 8.03% | 0.28 + 7.30% | 8.6% | 7.5% |
| Small window updating | One | 0.30 + 7.45% | 0.28 + 7.30% | 7.8% | 7.0% |
| Full-screen translucent | Two | 0.32 + 8.03% | 0.30 + 8.00% | 9.4% | 9.6% |
| Full-screen translucent | One | 0.32 + 7.43% | 0.30 + 7.43% | 8.2% | 7.8% |
| Full-screen translucent, blur | Two | 0.53 + 8.45% | 0.55 + 8.47% | 10.8% | 11.0% |
| Full-screen translucent, blur | One | 0.50 + 7.82% | 0.53 + 7.78% | 8.8% | 8.6% |
| Eight translucent | Two | 0.45 + 8.53% | 0.43 + 7.62% | 9.2% | 8.4% |
| Eight translucent | One | 0.40 + 7.97% | 0.40 + 7.67% | 8.2% | 7.7% |
| Eight translucent, blur | Two | 1.08 + 9.47% | 1.05 + 9.50% | 10.7% | 10.6% |
| Eight translucent, blur | One | 1.10 + 9.07% | 1.12 + 8.90% | 9.6% | 9.1% |
| Move and resize | Two | 1.00 + 8.68% | 0.97 + 8.22% | 8.5% | 7.5% |
| Move and resize | One | 1.00 + 8.40% | 0.95 + 8.22% | 7.7% | 7.2% |
| Open and close | Two | 0.45 + 9.30% | 0.45 + 8.70% | 8.9% | 8.6% |
| Open and close | One | 0.40 + 9.10% | 0.45 + 8.55% | 7.8% | 7.2% |

Where only part of the screen changes, the X server worked less. On two monitors it saved 0.7 points for the small window, 0.9 for eight translucent windows, and 0.5 for moving and resizing. Opening and closing measured 0.5–0.6 points less on either layout, but that is two ticks of CPU time in a 3.3-second scene, as the [blur reuse record](#recorded-blur-reuse-2026-10-04) explains. On one monitor, where the old full repaint covered half the area, the small window saved only 0.15 points and moving and resizing 0.2. GPU load fell by 0.5–1.1 points in those scenes, not by half: a 64×64 update still keeps the GPU 7–7.5% busy, so most of that load is a cost per frame, not per pixel. A full-screen translucent window changes the whole screen, and damage under any of eight overlapping blurred windows joins all their footprints, so those scenes repaint as much as before and cost the same. Compust's own CPU did not change in any scene beyond 0.05 points. Every scene held 60 frames per second; the old build skipped 4 vblanks once with eight blurred windows on one monitor, and the new build skipped none.

The new build presented no frame in any idle run. The old build's first idle run, the first scene of the sequence, presented 20 frames in its first three seconds, while desktop clients were still redrawing after the running compositor was stopped; its X server figure, 6.55%, raises that build's two-monitor idle mean. Open and close varied most between runs of one build, by up to 0.8 points, under three ticks, for the X server: in one old-build run, 19 of the first cycles took two frames to open and close, against one frame for every other open and close in both builds.

Blur is now the largest remaining cost of the eight-window scene. With region repaint it adds 0.6 points to Compust and 1.9 to the X server on two monitors, and 0.7 and 1.2 on one, because a change in the top window repaints the whole stack of blurred windows. That is the cost [step 4](ROADMAP.md#4-occlusion-and-blur-reuse) would reduce.

## Recorded blur reuse: 2026-10-04

After [blur reuse in step 4 of the rendering milestone](ROADMAP.md#4-occlusion-and-blur-reuse), the runner measured every scene on the same desktop with the region-repaint commit `fdc11224f3d3add229aa7197b6f00a79e5a9ea85`, whose compositor binary is identical to the new build of the [region record](#recorded-region-repaint-2026-10-04), and the blur-reuse commit `d1bd2f7f9ed7ef4d0e58cc71c76a308e3fb50040`. The builds alternated, twice with both monitors and twice with DP-2 alone; both had clean sources and used the same probe binary. Cells read as in the region record. The [records](benchmarks/2026-10-04/backdrop-amd-dwm/) include the [sequence](benchmarks/2026-10-04/backdrop-amd-dwm/sequence.sh).

| Scene | Monitors | Before | After | GPU before | GPU after |
| --- | --- | ---: | ---: | ---: | ---: |
| Idle | Two | 0.00 + 5.65% | 0.00 + 5.35% | 0.0% | 0.0% |
| Idle | One | 0.00 + 5.35% | 0.00 + 5.32% | 0.0% | 0.0% |
| Small window updating | Two | 0.28 + 7.40% | 0.28 + 7.32% | 6.2% | 6.2% |
| Small window updating | One | 0.30 + 7.35% | 0.25 + 7.32% | 7.0% | 7.0% |
| Full-screen translucent | Two | 0.30 + 7.90% | 0.30 + 7.85% | 8.6% | 8.5% |
| Full-screen translucent | One | 0.28 + 7.40% | 0.30 + 7.38% | 7.7% | 7.8% |
| Full-screen translucent, blur | Two | 0.50 + 8.30% | 0.35 + 8.12% | 5.8% | 5.5% |
| Full-screen translucent, blur | One | 0.43 + 7.62% | 0.30 + 7.45% | 8.5% | 8.0% |
| Eight translucent | Two | 0.40 + 7.62% | 0.40 + 7.65% | 6.5% | 6.5% |
| Eight translucent | One | 0.40 + 7.62% | 0.43 + 7.70% | 7.5% | 7.5% |
| Eight translucent, blur | Two | 1.02 + 9.22% | 0.50 + 8.00% | 8.6% | 6.5% |
| Eight translucent, blur | One | 1.02 + 8.82% | 0.50 + 8.05% | 8.9% | 7.4% |
| Move and resize | Two | 0.97 + 8.25% | 0.97 + 8.30% | 6.2% | 5.9% |
| Move and resize | One | 0.97 + 8.28% | 0.97 + 8.28% | 7.1% | 7.0% |
| Open and close | Two | 0.30 + 8.70% | 0.30 + 9.00% | 6.3% | 6.1% |
| Open and close | One | 0.45 + 8.55% | 0.30 + 8.70% | 7.0% | 7.2% |

Blur reuse helps where a blurred window's own content changes. With eight overlapping blurred windows and the top one changing, Compust's CPU halved, from 1.02% to 0.50%, and the X server worked 1.2 points less on two monitors and 0.8 on one. Blur now adds about 0.1 points to Compust and 0.35 to the X server over the same scene without blur, against 0.6 and 1.2–1.6 before. A full-screen translucent window with blur cost both processes 0.13–0.18 points less; each of its frames still copies the kept backdrop across the whole screen. The scenes without blur changed by two ticks of CPU time at most, apart from idle on two monitors: the old build's first idle run, again the first scene of the sequence, presented 2 frames and raised that mean. Every scene held 60 frames per second, and none skipped a vblank.

CPU figures count each process's scheduler ticks, 100 per second. Most scenes measure 20 seconds, where one tick is 0.05 points, but open and close measures only 3.3 seconds, where one tick is 0.3 points. Its X server figure on two monitors rose by exactly one tick in both runs, and the 0.5–0.6-point saving the region record reports for that scene is two ticks.

GPU load in the eight-window blur scene fell from 8.6% to 6.5% on two monitors and from 8.9% to 7.4% on one, but across scenes it does not follow the work: the full-screen blur scene showed less load than the same scene without blur. Compare it only between builds within one scene.

Kept backdrops are server pixmaps. In the full-screen blur scene, the pixmap bytes XRes attributes to Compust grew from 71.5 MB to 96.4 MB: one backdrop for the 3840×1080 translucent window and a 1920×1080 one for the desktop's Alacritty window, which has an alpha channel and lay under the benchmark's opaque backdrop. In the eight-window scene they grew from 60.5 MB to 75.1 MB.

## Recorded occlusion: 2026-10-04

After [occlusion in step 4 of the rendering milestone](ROADMAP.md#4-occlusion-and-blur-reuse), the runner measured every scene, including the new `covered` scene, on the same desktop with the commit that added that scene, `dc34c3b4e071a66fed23bc27ea35914b12bf446c`, whose compositor binary is identical to the new build of the [blur reuse record](#recorded-blur-reuse-2026-10-04), and the occlusion commit `899dfb64c3905095eb5a666a68a2d7ce89fbafef`. The builds alternated, twice with both monitors and twice with DP-2 alone; both had clean sources and used the same probe binary. Cells read as in the region record. The [records](benchmarks/2026-10-04/occlusion-amd-dwm/) include the [sequence](benchmarks/2026-10-04/occlusion-amd-dwm/sequence.sh).

| Scene | Monitors | Before | After | GPU before | GPU after |
| --- | --- | ---: | ---: | ---: | ---: |
| Idle | Two | 0.00 + 5.62% | 0.00 + 5.30% | 0.0% | 0.0% |
| Idle | One | 0.00 + 5.32% | 0.00 + 5.35% | 0.0% | 0.0% |
| Small window updating | Two | 0.25 + 7.35% | 0.25 + 7.22% | 6.2% | 6.0% |
| Small window updating | One | 0.25 + 7.30% | 0.23 + 7.18% | 7.0% | 6.9% |
| Full-screen translucent | Two | 0.30 + 7.90% | 0.25 + 7.70% | 8.6% | 7.8% |
| Full-screen translucent | One | 0.28 + 7.35% | 0.25 + 7.22% | 7.7% | 7.2% |
| Full-screen translucent, blur | Two | 0.35 + 8.10% | 0.28 + 7.90% | 5.4% | 4.9% |
| Full-screen translucent, blur | One | 0.32 + 7.43% | 0.25 + 7.28% | 8.0% | 7.5% |
| Eight translucent | Two | 0.40 + 7.68% | 0.38 + 7.55% | 6.5% | 6.6% |
| Eight translucent | One | 0.38 + 7.72% | 0.38 + 7.62% | 7.5% | 7.3% |
| Eight translucent, blur | Two | 0.47 + 8.07% | 0.45 + 8.18% | 6.5% | 7.1% |
| Eight translucent, blur | One | 0.45 + 8.00% | 0.45 + 7.78% | 7.5% | 7.5% |
| Covered | Two | 0.45 + 8.45% | 0.25 + 7.82% | 8.8% | 4.2% |
| Covered | One | 0.43 + 7.95% | 0.23 + 7.18% | 8.1% | 6.9% |
| Covered, blur | Two | 0.50 + 8.85% | 0.25 + 7.80% | 5.5% | 4.2% |
| Covered, blur | One | 0.47 + 8.12% | 0.25 + 7.20% | 8.3% | 6.9% |
| Move and resize | Two | 0.95 + 8.30% | 0.95 + 8.20% | 6.2% | 6.0% |
| Move and resize | One | 0.95 + 8.28% | 0.95 + 8.20% | 7.0% | 7.0% |
| Open and close | Two | 0.45 + 8.70% | 0.30 + 8.40% | 6.5% | 6.2% |
| Open and close | One | 0.45 + 8.70% | 0.30 + 8.55% | 7.0% | 7.2% |

Skipping hidden windows helps where an opaque window covers others. With eight translucent windows under an opaque full-screen window that changes every frame, Compust's CPU fell from 0.43–0.50% to 0.23–0.25%, and the X server worked 0.6–0.8 points less without blur and 0.9–1.05 with it; on two monitors, GPU load in that scene fell from 8.8% to 4.2%. The covered scene now costs the same with blur as without, because the hidden blurred windows no longer blur. They also drop their backdrops, which freed 14.6 MB of server pixmaps on two monitors.

In every other scene, the benchmark's opaque backdrop now hides the desktop's own windows and the root background. That saved the X server up to 0.2 points and freed the 8.3 MB backdrop that the blur reuse record found kept for the hidden Alacritty window. Eight blurred windows cost the X server 0.1 points more on two monitors in both runs and 0.2 less on one, although that scene now skips work it painted before; open and close differs by one tick at most. Every scene held 60 frames per second and none skipped a vblank. The old build's first idle run, again the first scene of the sequence, presented frames at its start.

## Recorded GPU renderer: 2026-10-04

The [GPU renderer](ARCHITECTURE.md#gpu-renderer) ran on the same desktop at the clean commit `ab665d31`, with the benchmark runner's `compust` and `compust-gl` entries side by side in each run, twice with both monitors and twice with DP-2 alone. Its log named the device it drew with: "AMD Radeon RX 9060 XT (radeonsi, gfx1200, ACO, DRM 3.64)". The [records](benchmarks/2026-10-04/gl-amd-dwm/) include the [sequence](benchmarks/2026-10-04/gl-amd-dwm/sequence.sh).

The probe's snapshot mode first showed the same scene of three translucent windows and an opaque one over one-pixel stripes, with blur, under each renderer. The two 3840×1080 captures differ by at most 2 levels in any channel: 76,896 pixels, 1.9% of the screen, differ by 1, and 12 by 2. Both [captures](benchmarks/2026-10-04/gl-amd-dwm/compust-gl-blur.ppm.gz) are in the records.

Each cell is Compust's CPU plus the X server's, as a mean of two runs. Every scene held 60 frames per second under both renderers without a skipped vblank. In the open-and-close scene, now 20 seconds long, each of the 2,400 windows the GPU renderer opened appeared one frame after its map request, within 17.0 ms, as under XRender.

| Scene | Monitors | XRender | GPU |
| --- | --- | ---: | ---: |
| Idle | Two | 0.00 + 5.40% | 0.00 + 5.35% |
| Idle | One | 0.00 + 5.38% | 0.00 + 5.40% |
| Small window updating | Two | 0.25 + 7.28% | 0.65 + 6.97% |
| Small window updating | One | 0.23 + 7.22% | 0.68 + 7.00% |
| Full-screen translucent | Two | 0.30 + 7.82% | 0.75 + 7.32% |
| Full-screen translucent | One | 0.25 + 7.32% | 0.78 + 7.00% |
| Full-screen translucent, blur | Two | 0.28 + 7.85% | 0.78 + 7.35% |
| Full-screen translucent, blur | One | 0.28 + 7.40% | 0.75 + 6.97% |
| Eight translucent | Two | 0.32 + 7.72% | 0.90 + 6.97% |
| Eight translucent | One | 0.35 + 7.68% | 0.90 + 7.00% |
| Eight translucent, blur | Two | 0.45 + 8.22% | 1.12 + 7.40% |
| Eight translucent, blur | One | 0.47 + 7.85% | 1.10 + 6.97% |
| Covered | Two | 0.25 + 7.65% | 0.72 + 7.38% |
| Covered | One | 0.25 + 7.22% | 0.62 + 6.95% |
| Covered, blur | Two | 0.25 + 7.75% | 0.70 + 7.35% |
| Covered, blur | One | 0.25 + 7.25% | 0.68 + 6.97% |
| Move and resize | Two | 0.95 + 8.28% | 2.02 + 9.15% |
| Move and resize | One | 0.95 + 8.22% | 1.98 + 9.07% |
| Open and close | Two | 0.60 + 7.84% | 1.42 + 8.12% |
| Open and close | One | 0.57 + 8.59% | 1.38 + 8.87% |

The GPU renderer moves work from the X server to Compust. Compust's own CPU rose from 0.23–0.47% to 0.62–1.12% in the scenes that compose windows, and the X server's fell by 0.22–0.88 points; their sum stayed within 0.25 points of XRender's. Moving and resizing cost 1.9 points more in total, and opening and closing 1.1 more: each new window pixmap, one per resize, is shared through DRI3, which costs round trips and, in glamor, can mean copying the pixmap into a buffer it can share. Compust's RSS rose from about 4 MiB to 66 MiB, almost all of it the GL driver. On this desktop the GPU renderer therefore saves nothing; a slower or busier X server, where XRender's work in the server dominates, is where it could.

A first run, at `a137a7db`, failed once: in its second two-monitor run, a window that the GPU renderer opened stayed black until the probe gave up. glamor holds back its GPU commands until the server idles or a fence triggers, so on a busy server the GPU could read a newly painted window, or the copy that sharing it needed, before those commands were sent, and no later damage repainted it. The fix triggers a SYNC fence before each frame; the opt-in [`tests/gpu.rs`](../tests/gpu.rs) reproduces the race with another client keeping the server busy, which left 4–14 of 100 shared pixmaps stale without the fence and none with it. The [failing run's records](benchmarks/2026-10-04/gl-first-amd-dwm/) are kept.

## Recorded desktop sessions on the RX 9060 XT: 2026-10-04

The [RX 9060 XT desktop](#recorded-reload-session-2026-10-03) ran the [hardware procedure](#run-the-desktop-checks-on-hardware) under four window managers. Each session was a dedicated XLibre 25.1.9 server on vt3, started by `startx` while the usual session stayed on vt2, with DP-2 as the only active output at 1920×1080 and 60 Hz. Every [server log](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/xorg.log) reports glamor on radeonsi with OpenGL 4.6 and TearFree enabled. The Xmonad 0.18.1, Openbox 3.6.1, and i3 4.25.1 sessions ran the clean commit `2311948f700930ee1f9f3b9ba9e6ec8e66c180cb`, which prepares 0.3.0-beta.1. The bspwm 0.9.12 session ran `526eb96`, two commits later, which changes the runner and the probe and leaves the compositor's source as it was. The compositor binaries were two builds of that source: `d455cea47e220def…` under Xmonad, and `05961c098ae05c04…`, which Cargo built together with the probe, under the others. The [records](benchmarks/2026-10-04/desktop-rx9060xt/) keep each session's reports without the screen captures.

Every run in the table passed every scenario.

| Window manager | Mode | Active Compust CPU | Active X server CPU | Frames in 10 s | Interval median / p95 / max |
| --- | --- | ---: | ---: | ---: | ---: |
| Xmonad | [Present](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/present/processes.csv) | 0.2% | 1.6% | 600 | 16.667 / 16.674 / 16.677 ms |
| Xmonad | [Direct](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/direct/processes.csv) | 0.1% | 1.5% | 600 Damage | — |
| Xmonad | [Effects](benchmarks/2026-10-04/desktop-rx9060xt/xmonad/effects/processes.csv) | 0.2% | 1.7% | 600 | 16.666 / 16.674 / 16.677 ms |
| Openbox | [Present](benchmarks/2026-10-04/desktop-rx9060xt/openbox/present/processes.csv) | 0.2% | 2.0% | 600 | 16.667 / 16.675 / 16.678 ms |
| Openbox | [Direct](benchmarks/2026-10-04/desktop-rx9060xt/openbox/direct/processes.csv) | 0.2% | 1.5% | 600 Damage | — |
| Openbox | [Effects](benchmarks/2026-10-04/desktop-rx9060xt/openbox/effects/processes.csv) | 0.3% | 2.1% | 600 | 16.667 / 16.675 / 16.678 ms |
| i3 | [Present](benchmarks/2026-10-04/desktop-rx9060xt/i3/present/processes.csv) | 0.3% | 2.1% | 600 | 16.667 / 16.675 / 16.677 ms |
| i3 | [Direct](benchmarks/2026-10-04/desktop-rx9060xt/i3/direct/processes.csv) | 0.2% | 1.6% | 600 Damage | — |
| i3 | [Effects](benchmarks/2026-10-04/desktop-rx9060xt/i3/effects/processes.csv) | 0.3% | 2.1% | 600 | 16.667 / 16.669 / 16.676 ms |
| bspwm | [Present](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/present/processes.csv) | 0.3% | 1.6% | 600 | 16.667 / 16.675 / 16.677 ms |
| bspwm | [Direct](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/direct/processes.csv) | 0.1% | 1.5% | 600 Damage | — |
| bspwm | [Effects](benchmarks/2026-10-04/desktop-rx9060xt/bspwm/effects/processes.csv) | 0.3% | 1.8% | 600 | 16.667 / 16.675 / 16.676 ms |

Present followed vblank exactly: every active-phase MSC advanced by one in all eight runs that present. In idle phases, Compust and the X server recorded no CPU ticks. Compust RSS stayed at 4,128–4,276 KiB and the X server's at 91,116–95,256 KiB, unchanged within each phase. The effects mode adds 180 ms fades, blur radius 4, and a surviving window at 50% opacity; it cost the X server at most 0.2 points more than the Present run.

The bspwm session ran twice, and the table shows the second run. In the first, the Present and direct runs passed and the effects run finished: its 600 frames are recorded, and its X server logged a normal exit at 16:20:31. Within the next minute the screen went black and the machine stopped responding. It was reset, and the files written in that session's last seconds never reached the disk: the probe's result line, the CPU record, and the end of the session log are empty. The logs do not say why. The journal ends as the next X server starts, and the shell history shows two `startx` attempts in those seconds, each with a report directory that the script cannot create, which stops it before it starts Compust. The [first session's records](benchmarks/2026-10-04/desktop-rx9060xt/bspwm-interrupted/) are kept as they were found. The second session ran forty minutes later without incident.

bspwm 0.9.12 sometimes keeps the tile of a window that no longer exists: one of the 32 that the probe destroys right after their map requests. On Xorg's Xvfb that happened in 2 of 8 rehearsals, where `xprop` confirmed that the window of the remaining node was gone. On hardware it happened in one of the five bspwm runs whose final capture survives: in the first session's Present run the surviving window kept half of the screen to the end, and in the other four it filled the screen. The probe accepts the leftover tile under bspwm as long as it shows the background.

A first Xmonad session was started with `SESSION_OUTPUT=DP=2`. `xrandr` ignored the unknown name, and the script turned both outputs off, so the session ran on a 320×200 root with no active CRTC. Every scenario still passed there: Present completed once per second, and Compust logged its one-second recovery, "Present did not finish a submission; replacing its buffers", twice in the Present run and three times in the effects run. The [records](benchmarks/2026-10-04/desktop-rx9060xt/outputs-off/) keep that session as the only one with every output off, and the script now stops when the named output is not connected.

These sessions use the probe's synthetic windows, one monitor, and ten-second phases. dwm, which this desktop's own session uses, has no private configuration in the runner, and physical hotplug, suspend and resume, and the GPU renderer were not part of them. With the same build in an ordinary session on this desktop, the frosted halo around Brave's right-click menus was gone, as the [weighted blur](ROADMAP.md#everyday-usability-per-window-rules-and-weighted-blur) intends; that check was by eye and has no record.

## Recorded shadows, focus, and fullscreen suspension (2026-10-04)

The dedicated feature probe passed in XRender and GL on the AMD Ryzen 5 5600X / Radeon RX 9060 XT desktop, with XLibre 25.1.9, Linux 7.2.8-arch1-1, modesetting with glamor and TearFree, and radeonsi. DP-2 alone was enabled at 1920×1080 and 60 Hz, depth 24; HDMI-1 was connected but disabled. There was no window manager: the probe controlled `_NET_ACTIVE_WINDOW` itself. The [archive](benchmarks/2026-10-04/features-rx9060xt/README.md) identifies the configuration, base `1969270257af37c092bad5d440d86ea74e680c01` plus recorded local changes, source and executable hashes, logs, and captures. The hashes matched the tested local files when the record was collected.

All 2,073,600 pixels in each shadow scene passed the independent reference within one RGB level, below the tolerance of two. XRender differed in 40 pixels and GL in 25. The [comparison between painters](benchmarks/2026-10-04/features-rx9060xt/features/gl/shadows-comparison.csv) found 45 differing pixels, also by at most one level. Both focus images were identical between painters; focus in both directions, `NONE`, and the absent-property fallback passed. The GL [compositor log](benchmarks/2026-10-04/features-rx9060xt/features/gl/compust.log) confirms the RX 9060 XT GPU painter without XRender fallback. The archived PNG captures preserve every RGB pixel of the original PPMs and were inspected visually.

Each measured phase lasted ten seconds after two seconds of warmup and requested 600 full-window redraws at 60 per second. Both composed phases recorded 600 Present completions and 600 overlay Damage events; both suspended phases recorded zero. The popup resumed composition, the new capture showed what was drawn while suspended, and removing the cover restored the background. Both compositor processes exited successfully after SIGTERM.

| Painter | State | Compust CPU | X server CPU | Compust RSS before/after |
| --- | --- | --- | --- | --- |
| [XRender](benchmarks/2026-10-04/features-rx9060xt/features/xrender/processes.csv) | Composed | 0.3% | 1.9% | 4,236 / 4,236 KiB |
| XRender | Suspended | 0.0% | 1.0% | 4,236 / 4,236 KiB |
| [GL](benchmarks/2026-10-04/features-rx9060xt/features/gl/processes.csv) | Composed | 0.6% | 1.4% | 71,112 / 71,112 KiB |
| GL | Suspended | 0.0% | 1.1% | 71,112 / 71,112 KiB |

CPU percentages use one core. At 100 accounting ticks per second, one tick over ten seconds is 0.1 percentage point; zero means no accumulated Compust ticks in that interval. RSS was unchanged during each measured phase. This record covers short synthetic fixtures on one display. Real fullscreen applications, window-manager focus behavior, other drivers and servers, multiple active monitors, shadow performance, and long-duration resource stability still require their own records.


## Recorded mixed-refresh monitors (2026-10-04)

The live RX 9060 XT / XLibre 25.1.9 / Xmonad 0.18.1 desktop passed nine monitor samples with HDMI-1 at 1920×1080 60 Hz and DP-2 at 1920×1080 180 Hz, for a 3840×1080 root. DP-2 was actually at 60 Hz before the test and was switched to 180 Hz. XRender with Present, direct XRender, and GL with Present each checked both monitors' opposite corners, the shared seam, and a managed window repaint while the primary output changed DP-2 → HDMI-1 → DP-2. The [record](benchmarks/2026-10-04/mixed-refresh-rx9060xt/README.md) identifies the source at `a3e6f31`, binaries, configurations, raw timings, CPU/RSS, XRes, and output states.

All pixels passed, all three compositor processes stopped cleanly, and the GL log confirmed hardware rendering without fallback. XRes counts stayed identical within each profile and owned pixmaps held 24,962,725 bytes in every sample. Present's MSC/UST clock followed the primary output, about 180 Hz or 60 Hz; the largest active completion interval was 16.675 ms. The original compositor binary and configuration were restored, leaving DP-2 primary at 180 Hz and HDMI-1 at 60 Hz.

Each sample measured three idle and three active seconds after warmup. Ordinary applications kept drawing, so idle phases still had frames. The marker requests only 60 updates/s and Compust's cap was 120 fps; this checks operation with different monitor rates, without certifying independent 180-fps output scheduling or panel latency. GL RSS rose by 2,640 KiB across renderer recreations despite stable XRes counts; three transitions cannot establish a long-term memory trend. This session did not repeat physical hotplug or qualify effects across the mixed-refresh layout.

## Recorded Xorg sessions on the Radeon Vega desktop (2026-10-05)

The [Radeon Vega desktop](#recorded-hardware-session-2026-10-03), an AMD Ryzen 5 5600GT whose integrated graphics (Cezanne) use radeonsi, moved from XLibre 25.1.9 to Xorg 21.1.24 from CachyOS. Both servers drive it with modesetting and glamor, which this machine's `/etc/X11/xorg.conf.d` selects. Xorg 21.1.24's modesetting has no TearFree, which the XLibre sessions had by default. Each run below was a dedicated server on vt3, started by `startx` from a text console while the usual i3 session stayed on vt2, with HDMI-1 alone at 1920×1080 and 60 Hz and DP-1 connected but disabled. Both ran the clean commit `ed2cdb04f828887231660d4c10729245b4a80b73` on Linux 7.2.9-1-cachyos; the compositor's SHA-256 was `cc8ad9e1e74afaf9…` and the probe's `5cbe6be39cbd9bd0…`. Beforehand, on the usual session's server, `compust --diagnose` reported the same extensions as XLibre, including DRI3 1.2 and Present 1.2, and the GPU crate's DRI3 tests passed against it with `COMPUST_GPU_DISPLAY=:0`; those two checks are not archived.

The [hardware procedure](#run-the-desktop-checks-on-hardware) under i3 4.25.1 passed every scenario in each mode, including framed title bars, the popup, the fullscreen restore, the return to the anchor's workspace, the wallpaper change, and the rapid lifecycle.

| Mode | Active Compust CPU | Active X server CPU | Frames in 10 s | Interval median / p95 / max |
| --- | ---: | ---: | ---: | ---: |
| [Present](benchmarks/2026-10-05/desktop-vega-xorg/i3/present/processes.csv) | 0.4% | 2.9% | 600 | 16.667 / 16.667 / 16.667 ms |
| [Direct](benchmarks/2026-10-05/desktop-vega-xorg/i3/direct/processes.csv) | 0.3% | 2.5% | 600 Damage | — |
| [Effects](benchmarks/2026-10-05/desktop-vega-xorg/i3/effects/processes.csv) | 0.4% | 2.6% | 600 | 16.667 / 16.667 / 16.667 ms |

Every active-phase MSC advanced by one in both runs that present. In idle phases, no process recorded a CPU tick. The effects run's idle phase saw one Present completion and one overlay Damage event, as did ten of the eleven earlier effects runs archived on hardware; the other two modes saw none. Compust RSS stayed at 4,336–4,364 KiB and the X server's at 100,176–104,772 KiB, unchanged within each phase except for 132 KiB the server gained during the direct run's active phase. These numbers do not compare directly with this machine's [XLibre session](#recorded-hardware-desktop-session-2026-10-03), which ran an older build under Xmonad with TearFree.

The [feature probe](benchmarks/2026-10-05/features-vega-xorg/README.md) passed in XRender and GL without a window manager. Each shadow scene's 2,073,600 pixels passed the independent reference within one level: 40 differed in XRender and 25 in GL, and the painters differed in 45, the same counts as on the [RX 9060 XT](#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04). Both focus images were identical between painters, and focus in both directions, `NONE`, and the absent-property fallback passed. The GL log names `AMD Radeon Graphics (radeonsi, renoir, ACO, DRM 3.64, 7.2.9-1-cachyos)` and records no fallback. Both composed phases recorded 600 Present completions and 600 overlay Damage events, and both suspended phases none; the popup resumed composition with a fresh capture, and removing the cover restored the background.

| Painter | State | Compust CPU | X server CPU | Compust RSS before/after |
| --- | --- | --- | --- | --- |
| [XRender](benchmarks/2026-10-05/features-vega-xorg/features/xrender/processes.csv) | Composed | 0.5% | 3.4% | 4,372 / 4,372 KiB |
| XRender | Suspended | 0.0% | 1.2% | 4,372 / 4,372 KiB |
| [GL](benchmarks/2026-10-05/features-vega-xorg/features/gl/processes.csv) | Composed | 1.1% | 1.7% | 66,180 / 66,180 KiB |
| GL | Suspended | 0.0% | 1.3% | 66,180 / 66,180 KiB |

The archives keep the CSV files, logs, configurations, and hashes. Their server logs omit the kernel command line and raw EDID and redact monitor serial numbers, and the feature captures are lossless PNG copies of the original PPM files. These sessions use synthetic windows, one monitor, and ten-second phases. Physical hotplug, other window managers, actual applications, and suspend and resume have no record on this server.

## Complete the hardware gates

Use a dedicated Xorg or XLibre test session with the intended window manager. Record the exact commit and build hashes, distribution, server version, GPU and driver, window-manager version/configuration, `compust --diagnose`, `xrandr --verbose`, and compositor configuration. Stop the existing compositor before starting Compust; retain the command needed to restore it. Do not run the scenario probe against a normal working session: it creates and destroys windows and switches workspaces. The monitor-sampling mode above moves only its own marker.

Run the [hardware session](#run-the-desktop-checks-on-hardware) for the probe's scenarios, wallpaper change, fades, and translucent blur. Then repeat the lifecycle, menus, fullscreen, and workspace scenarios with actual applications, and add decorated or reparenting window managers and session shutdown. Record which scenarios passed, their reproductions, and logs for every failure. Xmonad's checks do not qualify another window manager.

For step 2, run the [monitor-transition procedure](#sample-monitor-transitions) in both presentation modes on each environment proposed for support. It records connector names, modes, refresh rates, and the monitor layout around each physical disconnection and reconnection, and checks redraws across shared monitor edges. Virtual CRTC disable/enable is covered in automation and does not substitute for these connector tests. The session above covers one AMD/XLibre environment.

For step 3, measure idle and active CPU for both Compust and the X server, memory trends over repeated operations, and Present intervals where available. Include workload duration, window count, opacity/blur settings, and monitor refresh rates. Repeat the same Compust workload and configuration to measure variation and regressions. Publish evidence for each server/window-manager/driver combination before declaring it supported; beta distribution remains gated on that declared scope.
