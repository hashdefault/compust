# Roadmap

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

The goal is a minimal Rust compositor that becomes a practical choice for Xorg and XLibre users. “Better than picom” must eventually mean observable improvements in reliability, latency, resource use, or maintainability. Rewriting an existing feature in Rust is not, by itself, evidence of better performance.

## 0.1 foundation: implemented

The repository contains an executable compositor with XRender composition, fades, alpha transparency, blur, shape clipping, application redraw tracking, stacking, window resize handling, wallpaper properties, and optional Present copy scheduling. The test suite checks real server behavior on Xvfb. Documentation and contribution paths are available in English and pt-BR.

This milestone establishes a base for experiments. It does not establish desktop-wide compatibility or hardware performance.

## First beta: four steps

The first beta uses the XRender backend and declares support only for environments with recorded test evidence. It is intended for controlled community testing. Version 0.2.0-beta.1 was that beta, and 0.2.0-beta.2 and 0.2.0-beta.3 follow it; the [beta guide](BETA.md) lists the current declared scope.

| Step | Status | Required result |
| --- | --- | --- |
| 1. Window stability | Complete on Xvfb | Window lifecycle, menus, fullscreen transitions, and invalid properties have reproducible coverage without crashes or stale/invisible windows. |
| 2. Monitors and resources | Verified on Xvfb, one AMD/XLibre desktop, and one Intel/Xorg laptop, each with physical hotplug; other hardware pending | Resolution changes, monitor connection/disconnection, presentation recovery, and repeated resize resource use are verified. |
| 3. Real desktops | Xmonad scenarios recorded on nested servers, one AMD/XLibre desktop, and one Intel/Xorg laptop; Openbox and i3 scenarios on that laptop; other WMs and drivers pending | Xorg/XLibre sessions have recorded window-manager and driver coverage, plus CPU, memory, and frame-pacing measurements. |
| 4. Beta distribution | Published as v0.2.0-beta.1, v0.2.0-beta.2, and v0.2.0-beta.3, each for its declared scope | A versioned prerelease includes install/run instructions, known limits, verified artifacts, and a reproducible bug-report procedure. |

### 1. Window stability

Exercise rapid map/unmap/destroy sequences, interrupted fades, decorated windows, override-redirect menus, entering and leaving fullscreen, malformed properties, and windows disappearing during protocol requests. Start with deterministic Xvfb pixel regressions; record actual window-manager behavior in step 3.

**Acceptance:** each reproduced defect has a regression that fails without the fix; supported scenarios preserve the correct pixels and keep the compositor alive; the full suite, formatting, Clippy, and release build pass. Coverage below includes clients and frames, properties, rapid sequences, interrupted fades, extreme/off-screen shapes, and destruction before and between capture requests. The automated Xvfb gate is complete; actual window-manager and driver qualification remains in step 3.

### 2. Monitors and resources

Test RandR resolution changes and physical hotplug, supported presentation failure recovery, and repeated resizes with X-server resource accounting. Record the configuration and monitor arrangement used.

**Acceptance:** output recovers after each supported transition, presentation continues, and repeated operations do not produce unbounded growth in memory or server resources.

The Xvfb scenarios below pass. A native XLibre session on AMD hardware also passed physical unplugging and reconnection of both connectors, plus two-monitor mode and layout changes, in both presentation modes. Other drivers and servers, mixed refresh rates, and more than two monitors remain unverified; disabling a virtual CRTC does not establish physical behavior.

### 3. Real desktops

Run documented scenarios with real window managers on Xorg and XLibre. Record the server, window manager, GPU/driver, configuration, and exact commit. Measure idle and active CPU, memory, and frame pacing; fix failures in the environments proposed for beta support.

**Acceptance:** publish a compatibility matrix with evidence for each advertised environment, a reproducible measurement baseline, and remaining limitations. Untested driver/server combinations remain unqualified.

The [desktop qualification guide](DESKTOP_TESTING.md) documents the isolated runner, measurements, and hardware procedure. The recorded Xephyr/Xmonad runs start this step; they do not close its driver and physical-display requirements. A dedicated AMD/XLibre session now runs the probe's scenarios, wallpaper change, fades, and translucent blur on hardware. Actual applications, decorated or reparenting window managers, Xorg on hardware, and other drivers remain open.

### 4. Beta distribution

Publish a versioned beta prerelease with build or binary installation instructions, configuration examples, startup/shutdown guidance, checksums for distributed artifacts, known limitations, and a bug-report template including diagnostics and reproduction steps.

**Acceptance:** a tester can install and run the exact release, return to their previous compositor, and report a failure from the supplied instructions. CI passes for the release commit and the previous three gates are satisfied for its declared support scope.

Version 0.2.0-beta.1 completed these four acceptance gates for its declared scope. GPU backend expansion and advanced effects can follow the first beta. The [next milestone](#next-measure-and-reduce-rendering-work) is rendering work; widening the beta's hardware coverage continues beside it as reports arrive.

## Progress and verification

### Everyday usability: configuration discovery and reload

Beta use showed that changing `fade_ms` required a restart and that a configuration was read only when `--config` named it. Without `--config`, Compust now reads the first `compust/compust.toml` in `$XDG_CONFIG_HOME` (default `~/.config`), then in `$XDG_CONFIG_DIRS` (default `/etc/xdg`), and `--check-config` names the file it found. SIGUSR1 reloads the configuration a restart would read. A file that cannot be read or is invalid is logged, and the running configuration stays. A reloaded `fade_ms` applies to every transition that starts afterward, and a blur or vsync change replaces the renderer once Present has released its buffer. A rejected Present submission now marks the extension unavailable for the session instead of turning off `vsync`, so a reload cannot bring back a path the server refused.

Seven X11 tests in [reload.rs](../tests/cases/reload.rs) cover an opacity reload; a reloaded fade duration closing a window opened before it; rejected reloads with an out-of-range value, a syntax error, and a removed file; blur toggled sixteen times in each presentation mode with unchanged XRes counts and pixmap bytes; switching between Present and direct copying; and a reload after a Present rejection. Each failed when its part of the behavior was disabled in a temporary copy. Unit and CLI tests cover the search order, a dangling link, and `--config` taking precedence over discovery.

The suite ran with XLibre's Xvfb 25.1.9, because Xorg's conflicts with the XLibre packages on that machine. The seven existing tests that reconfigure the RandR CRTC fail there before and after this change: that Xvfb reports a 1280×1024 CRTC on a 320×240 screen and rejects `SetCrtcConfig` with `BadValue`. The other 58 X11 tests, 12 unit tests, and 6 CLI tests pass, with formatting and strict Clippy; CI uses Xorg's Xvfb. A [live session](DESKTOP_TESTING.md#recorded-reload-session-2026-10-03) on a third machine, XLibre with a Radeon RX 9060 XT under dwm, reloaded blur, fades, and vsync and rejected an invalid file across one and two monitors. Every Present interval spanned one vblank, and repeated states held identical owned pixmap bytes.

### Local beta testing: complete Xmonad borders

Daily use after `v0.2.0-beta.1` exposed missing right and bottom borders. On a window without a client bounding shape, `ShapeGetRectangles` returned dimensions shorter than the captured pixmap by one border width. Compust now uses the full pixmap bounds for these windows and preserves explicit client shapes.

The existing [off-screen border regression](../tests/cases/shapes.rs) failed before the fix and passes afterward. [Border regressions](../tests/cases/borders.rs) cover focus color updates, resize with a changed border width, and removal of a custom shape. All 68 tests, formatting, strict Clippy, and the release build pass. In the local Xmonad session, both Alacritty windows retain all four 2-pixel border strips in focused and unfocused states. This continues beta testing; the animation tasks below remain planned.

### Local beta testing: stale window after a restored resize

On the Intel/Xorg laptop with i3, connecting or disconnecting an HDMI cable froze every tiled window until another window opened. i3 re-tiled twice in quick succession and ended at the original sizes. The X server allocates a new window pixmap on each resize, but Compust compared only the window's current size with the captured one, saw no change, and kept compositing the old pixmap, which no longer received drawing or Damage. A recording showed a tiled test window frozen for 26 seconds, from the unplug until a new window resized it, while an override-redirect window kept updating.

Compust now also compares the size carried by each `ConfigureNotify` and recaptures when it differs. A [regression](../tests/cases/stability.rs) resizes and restores a window under a server grab, then repaints it; it failed before the fix and passes afterward. With the fix, the same cable test left the tiled window updating through the unplug and the reconnection. All 70 tests, formatting, strict Clippy, and the release build pass.

The defect is not specific to hotplug or i3: any window manager that resizes a window and restores it before Compust handles the first event can trigger it. It is present in 0.2.0-beta.1 and 0.2.0-beta.2. The monitor-transition runner did not detect it, because it checked only its own override-redirect marker. It now also repaints a managed window in place during every sample; with that check, the 0.2.0-beta.2 binary fails after an output is turned off under i3 and the fixed compositor passes.

### Local beta testing: Present timeout

Compust reuses its Present buffer only after the completion and idle events of the previous submission. Outside a RandR change, a lost event left the screen frozen while the compositor kept running. No such loss has been observed on hardware; the risk was found by reviewing the code. A submission that reports nothing within one second is now abandoned: Compust replaces its buffers and repaints the frame once. If the next submission also times out, it waits for new damage before painting again.

A new regression in [presentation.rs](../tests/cases/presentation.rs) withholds one submission's events without any monitor change. It failed before the fix, with the screen left on the previous frame, and passes afterward. All 69 tests, formatting, strict Clippy, and the release build pass. Virtual-terminal switching and suspend/resume, where such a loss is most likely, remain untested on hardware.

### Client/frame lifecycle: implemented

Client association now follows late creation and removal of `WM_STATE`, new descendants under an existing frame, reparenting between mapped frames and the root, and client destruction. Frame opacity retains precedence. A queued opacity event for an already destroyed client no longer makes its surviving frame disappear.

Five pixel regressions in [client_lifecycle.rs](../tests/cases/client_lifecycle.rs) exercise these transitions against the actual compositor binary. The four lifecycle scenarios failed before the fix; disabling the new property handling and client-only `BadWindow` recovery also reproduced the stale-opacity and vanished-frame failures.

### Step 1 started: property validation and rapid lifecycle scenarios

`WM_STATE` now requires its declared type, 32-bit format, and exactly two values. Window opacity requires a single 32-bit `CARDINAL`. Empty, truncated, overlong, or incorrectly typed/formatted properties are ignored; invalid frame opacity falls back to valid client opacity. Five regressions in [properties.rs](../tests/cases/properties.rs) failed before the fix and now pass, including recovery after replacing malformed client state with valid data.

Four additional scenarios in [stability.rs](../tests/cases/stability.rs) pass: 32 queued map/unmap cycles followed by a moved remap, destruction with queued opacity/configure/shape events, removal of an override-redirect popup, and expansion/restoration of fullscreen-sized geometry. They check rendered pixels and that the compositor remains alive. Fullscreen coverage here changes window geometry directly on a fixed Xvfb screen; real window-manager fullscreen behavior belongs to step 3, and monitor reconfiguration to step 2.

### Step 1 continuation: fades, shapes, and destruction

Three defects were reproduced and fixed. Remapping during close now captures the new content and resumes opening from the current opacity. A destroyed window stays below its former upper neighbor while fading, even after another stacking event. Shapes remain in local coordinates; the XRender clip origin applies the window position without prematurely saturating negative coordinates and leaving black pixels.

The remap and stacking regressions in [fades.rs](../tests/cases/fades.rs) and the extreme-coordinate regression in [shapes.rs](../tests/cases/shapes.rs) failed before the fixes and passed afterward. Additional coverage includes destruction during opening, empty and disjoint shapes, 20-pixel borders, off-screen blur clipping, and a 4096×2048 window moved fully off-screen and back. A unit test checks continuity, monotonicity, and the exact endpoint when reopening a fade.

Three scenarios in [destruction.rs](../tests/cases/destruction.rs) check 32 queued create/map/destroy sequences before capture, plus destruction with a shape or geometry event ahead of the destroy notification. A server grab enforces the ordering; these tests exercise existing recovery without broadening ignored X11 errors. They do not force destruction into each gap between the internal capture requests.

This batch brought the suite to 44 passing tests. The remaining capture-request boundaries are covered by the cases below.

### Step 1 complete: destruction between capture requests

Three tests in [capture_races.rs](../tests/cases/capture_races.rs) exercise eleven request boundaries: window attributes, geometry, pixmap naming, picture creation, client event subscription, `WM_STATE`, client tree discovery, shape event subscription, damage creation, shape rectangles, and opacity. A test-only loopback proxy pauses the selected request, completes a checked destruction through a separate Xvfb connection, then forwards the original bytes. Replies and events come from the real server; there are no production hooks or timing sleeps.

Every case waits for a surviving marker to render, checks the underlying window's pixels and compositor liveness, and probes the captured pixmap, picture, and damage identifiers to verify release. The existing capture recovery passed without production changes or broader error suppression. In temporary copies, disabling picture cleanup made the picture case fail with a still-allocated resource; disabling capture recovery made the compositor exit and the survivor case fail.

This completed step 1 with 47 passing tests: six unit, three CLI, and thirty-eight X11 integration tests. Formatting, strict Clippy, and the release build passed. Real desktops and physical hotplug still require their own evidence.

### Step 2: virtual monitor transitions, presentation recovery, and resources

Four tests in [monitors.rs](../tests/cases/monitors.rs) exercise real RandR requests with Present enabled and with direct XRender copying. The single Xvfb output is disabled, the root shrinks from 320×240 to 240×180, then the original size and CRTC configuration are restored. Sixteen measured cycles preserve the survivor's pixels, exact XRes resource counts, and full reported bytes of compositor-owned pixmaps. Separate cases disable and restore the output without changing root size. Disabling renderer replacement in a temporary copy makes both resize cases fail.

Four lifecycle tests in [resources.rs](../tests/cases/resources.rs) repeat mapped-window resizing and map/destroy sequences 32 times per rendering mode after warmup. Restored scenes retain exactly the same X-server resource counts and full reported pixmap bytes: 654,401 for the window scenarios and 878,401 for the larger monitor-test window. Disabling picture cleanup in a temporary copy makes all four lifecycle cases fail on their first measured cycle.

A fifth resource test holds extra pixmap references from another client. The full allocation total stays unchanged, while the old reference-weighted attribution drops from 641,066 to 429,600 bytes. The fixture uses XRes 1.2 `QueryResourceBytes` and verifies complete size coverage; `QueryClientPixmapBytes` is unsuitable for strict allocation comparisons while Present references change. Recorded Linux compositor RSS endpoints were unchanged or increased by one 4 KiB page. These finite checks detect growth in the exercised workloads; they do not establish a general memory bound or hardware GPU-memory usage.

Three tests in [presentation.rs](../tests/cases/presentation.rs) cover rejected submissions. A test proxy substitutes an incompatible pixmap in one Present request, producing a real server `BadMatch`. Previously the compositor exited. It now matches that exact submission, checks that its original back buffer and output still exist with compatible screen/depth, and continues through XRender for the rest of the session, including later root resizes. Invalid pixmap and window errors still terminate the compositor. Missing completion events outside monitor reconfiguration, covered below, and other presentation errors are outside this recovery policy.

The full suite now has 59 passing tests: six unit, three CLI, and fifty X11 integration tests. Formatting, strict Clippy, the release build, and documentation checks pass. Physical hotplug, multiple-monitor layouts, and hardware measurements were still open at that point; see the hardware session below. The automated configuration is one virtual output at 320×240, temporarily 240×180, with `fade_ms = 0`, `blur_radius = 0`, and each vsync mode.

### Step 3 started: Xmonad on nested Xorg and XLibre

The first [desktop records](benchmarks/2026-10-03/) exercised Xmonad 0.18.1 with Xorg Xephyr 21.1.24 and XLibre Xephyr 25.1.9 at commit `1409a919dd0c91ccaad3fbb323df4b33628554fd`. They covered managed tiling, popup removal, EWMH fullscreen/restore, workspace return, rapid lifecycle, survivor redraw, and SIGTERM shutdown with Present enabled.

The [desktop runner](../tools/desktop-check.sh) now accepts `present` or `direct`. Its probe observes overlay Damage for readiness and direct redraws, requires Present samples only for that path, and rejects mismatched presentation settings. Startup and scene checks use notifications with deadlines instead of timed polling. Reports record source changes and hashes alongside the base commit.

All four server/path combinations passed at base `cb0796c8e42a95d3c80ab11c557b75809f791d43` plus the archived probe/runner changes. The [bilingual qualification guide and raw evidence](DESKTOP_TESTING.md#recorded-baseline-2026-10-03) record ten-second idle/active phases, separate compositor/server CPU, unchanged RSS endpoints, Present intervals, and two negative checks. The full 59-test suite, formatting, strict Clippy, and release build pass. This completes the isolated baseline increment, not step 3's hardware gate. Existing Xmonad diagnostics and untested environments are recorded in the guide.

### Step 2 on hardware: physical hotplug on XLibre with AMD

A native XLibre 25.1.9 session with the modesetting driver, the amdgpu kernel driver, an AMD Ryzen 5 5600GT (Radeon Vega graphics, Mesa 26.2.4), and Xmonad 0.18.1 ran the new [monitor-transition runner](../tools/hotplug-check.sh). HDMI-1 (primary, right) and DP-1 (a DisplayPort-to-VGA adapter, left) both used 1920×1080 at 60 Hz. The [desktop probe](../examples/desktop_probe.rs) keeps one override-redirect marker. After each transition, it checks redraws near opposite corners of every active monitor and across each shared monitor edge, then records the RandR topology, ten-second idle and active phases, and the compositor's XRes accounting.

The first run found a defect. Changing HDMI-1 to 1280×720 froze output in Present mode: a CRTC reconfiguration and root resize happened while one submission was pending, and the server never sent its completion or idle event. Compust waited for those events before rebuilding its buffers. An [instrumented reproduction](benchmarks/2026-10-03/hardware/stall/diagnosis/compust-debug.log) recorded the missing events. A RandR change now replaces the renderer without waiting, and a same-size root `ConfigureNotify` no longer cancels a replacement requested earlier in the same event batch. Two regressions in [presentation.rs](../tests/cases/presentation.rs) withhold a submission's events with a never-triggered SYNC wait fence. Both failed before the fix; the output-toggle case also failed with only the first change.

With the fix, both presentation modes passed fifteen samples: the baseline, a 1280×720 mode and its restoration, a vertical 1920×2160 layout and its restoration, DP-1 off and on, and, for each connector, physical unplugging with the CRTC still assigned, `xrandr --auto`, reconnection, and restoration. Across 8,988 active-phase intervals, Present had a 16.667 ms median and p95 and a 16.670 ms maximum. While the marker updated 60 times per second, Compust used 0.3–0.5% of one core and Xorg 3.2–4.4%. Repeated topologies reported identical owned-pixmap bytes and resource counts, and Compust RSS stayed within 3,760–3,896 KiB. Details and raw evidence are in the [desktop qualification guide](DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03).

The full suite now has 61 passing tests: six unit, three CLI, and fifty-two X11 integration tests. Formatting, strict Clippy, and the release build pass. This qualifies monitor transitions only in the recorded environment, with fades and blur disabled. Intel and NVIDIA drivers, Xorg on hardware, mixed refresh rates, more than two monitors, and the step 3 desktop scenarios on hardware remain open.

### Step 3 on hardware: Xmonad scenarios and effects on AMD/XLibre

The probe gained a wallpaper scenario, which sets and removes `_XROOTPMAP_ID` on the empty workspace, and an `--opacity` option for its measured window. The runner's new `effects` mode uses the default 180 ms fades and blur radius 4 with a 50% translucent window, and the runner can now target an existing dedicated server. [`hardware-session.sh`](../tools/hardware-session.sh) runs all three modes as the client of a new X server started from a text console.

On the AMD/XLibre machine, a dedicated session with HDMI-1 alone at 1920×1080 passed every scenario in all three modes. Present completions followed vblank exactly: 600 frames at 16.667 ms, with Compust at 0.3% and Xorg at 3.1% of one core. The effects mode exposed the main performance problem. Blur behind a full-screen translucent window took 200–217 ms per frame while Xorg used 93.7%. Glamor accelerates only nearest and bilinear filtering, so the convolution filter used by Compust's blur falls back to the CPU on every redraw. Nested Xephyr at 1280×800 showed the same limit, 85 frames in ten seconds. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-hardware-desktop-session-2026-10-03) has the measurements and limits.

This completes the probe's scenarios in one hardware environment, with Xmonad and synthetic windows. Actual applications, decorated or reparenting window managers, server shutdown under a running compositor, Xorg on hardware, and other GPUs remain open. Blur needs an implementation that glamor can accelerate before it can be recommended on glamor-based drivers. Formatting, strict Clippy, the 61-test suite, and the release build pass.

### Blur on the GPU path

The step 3 measurement showed that glamor renders the convolution filter on the CPU. Blur now builds a pyramid instead: each level halves the area around a translucent window with bilinear sampling, and the coarsest level is scaled back up into the window's shape. Glamor accelerates these transforms. The configured radius rounds to 2, 4, 8, or 16 pixels. Level buffers exist only when blur is enabled and replace a full-screen scratch buffer.

On the AMD/XLibre desktop, effects mode went from 49 to 599 frames in ten seconds, with every Present interval at one vblank. Xorg used 4.1% of a core instead of 93.7%. Nested Xephyr went from 85 to 598–599 frames while using about 20% of a core instead of 86%. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03) has the records.

A new regression blurs a black/white edge aligned with every level at radii 4 and 16. It requires a monotonic transition centered on the edge, with distant pixels unchanged; shifting one pass by a pixel makes it fail. The existing stripe and off-screen shape tests pass unchanged, and three unit tests cover radius rounding and pyramid bounds. The full suite has 65 passing tests: nine unit, three CLI, and fifty-three X11 integration tests.

### Step 4: first beta prerelease

Version 0.2.0-beta.1 is published as a GitHub prerelease. The [beta guide](BETA.md) declares its scope: XLibre 25.1.9 with Xmonad 0.18.1 on the recorded AMD/glamor machine, with one or two 1080p monitors at 60 Hz. It also covers installation with checksum verification, startup with Xmonad or `~/.xinitrc`, returning to the previous compositor, known limits, and the information a bug report needs. The [bug template](../.github/ISSUE_TEMPLATE/bug.yml) asks for the same details.

[`package.sh`](../tools/package.sh) builds a release from a clean export of a revision with the pinned toolchain. It removes local paths from the binary, records `BUILDINFO`, and writes a deterministic tarball with `SHA256SUMS`; two runs on the build machine produced identical archives. The packaged binary printed its version, refused to start beside another compositor, exited successfully after SIGTERM, and exited with a connection error when its X server stopped. With that binary in place, the nested desktop checks passed in all three modes on XLibre Xephyr, and CI passed for the release commit.

The hardware records used the same source built together with the desktop probe, which only adds X-Resource support to x11rb; the released binary is built alone. Beta reports from other environments decide what a later release can declare.

### Steps 2 and 3 on a second machine: Xorg with Intel

A laptop with Linux Mint 22.3, native Xorg 21.1.11, the modesetting driver with glamor, Intel Iris Plus G1 graphics (i915, Mesa 25.2.8), and Xmonad 0.17.2 repeated the qualification procedures. It is the first record of Xorg on hardware and of an Intel driver. The nested checks passed in all three modes. On the ordinary desktop, both presentation modes passed a baseline, a change of the 1366×768 panel to 1280×720, and its restoration, with identical resource accounting before and after. A dedicated session then passed every probe scenario in Present, direct, and effects modes.

Present followed the panel's 60.06 Hz refresh, with a 16.650 ms median. One interval in each of the two dedicated Present runs spanned two vblanks. With blur behind a full-screen translucent window, Xorg used 4.4% of a core against 4.3% without it, so the pyramid blur stays on the GPU here too. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-intelxorg-sessions-2026-10-03) has the measurements and limits.

The laptop has one display, so physical hotplug was not tested there. The dedicated session ran the Present timeout change above on top of `ffd0b13`; the other runs used that commit unchanged. The released 0.2.0-beta.1 binary was not tested on this machine, so the beta's declared scope is unchanged.

### Second beta prerelease

Version 0.2.0-beta.2 carries the two fixes found after the first beta, the complete Xmonad borders and the Present timeout, and adds the Intel/Xorg laptop to the declared scope. The [beta guide](BETA.md) lists both qualified combinations, and the [release notes](releases/v0.2.0-beta.2.md) list the changes.

The Intel dedicated session ran the same source as this release, built together with the desktop probe; the AMD records predate both fixes. Neither machine's physical monitor sessions were repeated for this release.

Two [`package.sh`](../tools/package.sh) runs on the Intel laptop produced identical archives. The packaged binary printed its version, needs no glibc symbol newer than 2.34, refused to start beside another compositor, exited successfully after SIGTERM, and exited with a connection error when its X server stopped. With that binary in place, the nested desktop checks passed in all three modes on Xorg Xephyr 21.1.11.

### Step 3 with a second window manager: Openbox

The desktop runner and probe now accept Openbox, the first decorated, reparenting, stacking window manager tested. `WINDOW_MANAGER=openbox` starts it with a private configuration, and the probe's `--layout stacking` adds three scenarios: each client sits in a reparenting frame with a painted title bar, two overlapping windows come to the front as each is activated, and an iconified window leaves the screen and returns. The existing scenarios were adjusted for windows that keep their size. Xmonad runs are unchanged.

On the Intel/Xorg laptop, Openbox 3.6.1 passed the nested checks, the monitor-transition samples on an ordinary desktop, and a dedicated hardware session, each in every mode. No compositor change was needed. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-openbox-sessions-2026-10-03) has the measurements, an Openbox startup behavior the probe works around, and the limits.

This is evidence for one stacking window manager with synthetic windows. Interactive move and resize, window-manager menus, and actual applications still lack recorded scenarios, and the 0.2.0-beta.2 declared scope is unchanged.

### Step 3 with a third window manager: i3

The runner and probe also accept i3, a tiling window manager that reparents clients into framed title bars. `WINDOW_MANAGER=i3` starts it with a private configuration. Two probe options cover what differs from Xmonad: `--frames` requires the painted title bar outside the stacking layout, and `--workspace-anchor` maps a small window that i3 assigns to a second workspace, since i3 keeps no empty unfocused workspaces.

On the Intel/Xorg laptop, i3 4.23 passed the nested checks, the monitor-transition samples on an ordinary desktop, and a dedicated hardware session, each in every mode, again without a compositor change. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-i3-sessions-2026-10-03) has the measurements and limits. The 0.2.0-beta.2 declared scope is unchanged.

### Step 2 on a second machine: physical hotplug on Xorg with Intel

With an external 1920×1080 display on its HDMI connector, the Intel/Xorg laptop ran the monitor-transition procedure in both presentation modes: mode and layout changes, the output turned off and on, and the cable physically unplugged and reconnected. All 22 samples passed, repeated topologies reproduced identical XRes accounting, and no Present timeout was logged. The panel and the external display differ in resolution and by 0.06 Hz in refresh rate. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03) has the measurements and limits.

This is the second environment with physical hotplug evidence, and the first on Xorg and on an Intel driver. It ran after 0.2.0-beta.2, whose beta guide still lists hotplug on Intel as untested.

### Suspend and resume on Intel/Xorg

The Intel/Xorg laptop was suspended to RAM once in each presentation mode, with samples before and after. All six samples passed, Present pacing kept its 16.65 ms median, the compositor's owned pixmap bytes were identical throughout, and no Present timeout was logged. The [desktop qualification guide](DESKTOP_TESTING.md#recorded-suspend-and-resume-2026-10-03) has the records. Together with the virtual-terminal switches of the dedicated sessions, this covers the two cases the Present timeout entry above left untested, on this one machine; neither produced a lost completion there.

### Third beta prerelease

Version 0.2.0-beta.3 carries the fix for windows left stale after a restored resize, which both earlier betas have. Its declared scope adds what the Intel/Xorg laptop recorded after 0.2.0-beta.2: Openbox and i3, an external display with physical hotplug, and suspend and resume. The [beta guide](BETA.md) lists the scope, and the [release notes](releases/v0.2.0-beta.3.md) list the changes.

Those records predate the fix and use synthetic windows; the fix itself was confirmed with a recorded cable test under i3 and by the regression test. No hardware session was repeated with the release binary.

Two [`package.sh`](../tools/package.sh) runs on the Intel laptop produced identical archives. The packaged binary printed its version, needs no glibc symbol newer than 2.34, refused to start beside another compositor, exited successfully after SIGTERM, and exited with a connection error when its X server stopped. With that binary in place, the nested desktop checks passed in all three modes with Xmonad, Openbox, and i3 on Xorg Xephyr 21.1.11, using three-second phases.

### Compatibility matrix

| Environment | Verified coverage | Evidence / limits |
| --- | --- | --- |
| Xvfb 21.1.24 on CachyOS, 320×240×24, XRender and Present 1.2 | Clients/frames, properties, rapid sequences, interrupted fades, shapes, queued destruction events, and eleven capture-request boundaries; all 47 tests pass | Recorded 2026-10-02. Capture races use `fade_ms = 0`, `blur_radius = 0`, and default vsync. Earlier fade/shape cases also use `fade_ms = 1000`, `blur_radius = 4`, or `vsync = false` as described above. Hierarchies are created directly, without real window-manager or GPU qualification. |
| Same Xvfb, one virtual output, RandR and XRes | Root shrink/restore, CRTC disable/restore, rejected Present submission, and repeated resource accounting; all 59 tests pass | Recorded 2026-10-02 (local time). Server resource counts and bytes are checked at matching rendered states; physical hotplug and multiple monitors are outside this virtual setup. |
| Xorg 21.1.11 native on Linux Mint 22.3, modesetting + i915, Intel Core i3-1005G1 (Iris Plus G1, Mesa 25.2.8), Xmonad 0.17.2, one 1366×768 panel at 60 Hz | Nested checks; panel mode change and restoration in both presentation modes; probe desktop scenarios in Present, direct, and effects modes; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-intelxorg-sessions-2026-10-03). Physical hotplug was recorded later with an [external display](DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03). |
| Same Intel/Xorg laptop with Openbox 3.6.1 | Nested checks, panel mode change, and probe desktop scenarios in all modes, including decorated frames, restacking, and iconify | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-openbox-sessions-2026-10-03). One [suspend and resume](DESKTOP_TESTING.md#recorded-suspend-and-resume-2026-10-03) per presentation mode also passed. |
| Same Intel/Xorg laptop with i3 4.23 | Nested checks, panel mode change, and probe desktop scenarios in all modes, including framed title bars | Recorded 2026-10-03 with synthetic windows in the default split layout. [Results and limitations](DESKTOP_TESTING.md#recorded-i3-sessions-2026-10-03). |
| Xorg with a real window manager and AMD/NVIDIA drivers | Pending | Requires a recorded server, window manager, driver, configuration, and commit. |
| XLibre with Intel/NVIDIA drivers or other AMD configurations | Pending | Requires the same environment evidence; one AMD session does not establish other drivers. |
| Xorg Xephyr 21.1.24 + Xmonad 0.18.1, nested in Xvfb, 1280×800×24 | Desktop scenarios with wallpaper change, idle/active CPU and RSS, Present, direct XRender, and effects modes, shutdown | Recorded 2026-10-03: [baseline](DESKTOP_TESTING.md#recorded-baseline-2026-10-03) with fade/blur disabled and an [effects baseline](DESKTOP_TESTING.md#recorded-effects-baseline-2026-10-03). No physical display or driver qualification. |
| XLibre Xephyr 25.1.9 + Xmonad 0.18.1, same virtual layout | Same desktop scenarios and measurements in all three modes | Recorded 2026-10-03; same configuration and limitations. Nested server results do not qualify an XLibre hardware session. |
| XLibre 25.1.9 native, modesetting + amdgpu, AMD Ryzen 5 5600GT (Radeon Vega, Mesa 26.2.4), Xmonad 0.18.1, HDMI + DP-to-VGA at 1920×1080 60 Hz | Mode, layout, and output changes; physical unplugging and reconnection of both connectors; Present and direct XRender; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with fade/blur disabled while the desktop's own applications ran. [Results and limitations](DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03). The desktop scenario probe was not run in this session. |
| Dedicated XLibre 25.1.9 session on the same AMD machine, HDMI-1 alone at 1920×1080 60 Hz, glamor, default TearFree | Probe desktop scenarios with wallpaper change; Present, direct XRender, and effects (fades, translucency, blur); CPU, RSS, and Present pacing | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-hardware-desktop-session-2026-10-03). Convolution blur fell to about five frames per second behind a full-screen translucent window; the [pyramid blur](DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03) keeps 60. |
| XLibre 25.1.9 native, modesetting, AMD Ryzen 5 5600X + Radeon RX 9060 XT (Navi 44, radeonsi, Mesa 26.2.4), dwm 6.8, DP-2 + HDMI-1 at 1920×1080 60 Hz | Output on/off and layout changes with configuration reloads of blur, fades, and vsync; Present and direct XRender; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with synthetic windows while the desktop's own applications ran. [Results and limitations](DESKTOP_TESTING.md#recorded-reload-session-2026-10-03). No physical unplugging; the desktop scenario probe was not run. |

Reproduce the lifecycle checks with the repository's pinned toolchain and Xvfb installed. The [CI runs](https://github.com/hashdefault/compust/actions/workflows/ci.yml) record results against each exact commit; include the revision printed below in local reports.

```sh
git rev-parse HEAD
cargo test --locked --test x11 client_lifecycle
cargo test --locked --test x11 properties
cargo test --locked --test x11 stability
cargo test --locked --test x11 fades
cargo test --locked --test x11 shapes
cargo test --locked --test x11 destruction
cargo test --locked --test x11 capture_races
cargo test --locked --test x11 monitors
cargo test --locked --test x11 resources
cargo test --locked --test x11 presentation
```

Set `XVFB=/path/to/Xvfb` when the server is outside `PATH`.

### Remaining acceptance work

The probe's scenarios pass on one AMD/XLibre session and one Intel/Xorg session with Xmonad, and on the Intel/Xorg laptop with Openbox and i3. Extend hardware testing to further window managers, actual applications, NVIDIA drivers, and Xorg with AMD. Record server, driver, configuration, and commit with every report. Cover reparenting after startup, rapid map/unmap/destroy sequences, decorated and override-redirect windows, menus, fullscreen transitions, wallpaper tools, and session shutdown.

Repeat physical hotplug and multiple-monitor layouts with other drivers and servers, mixed refresh rates, and more than two monitors, and measure longer-running memory and presentation behavior. The AMD/XLibre and Intel/Xorg monitor sessions, virtual RandR transitions, recovery from rejected or unfinished Present submissions, and repeated XRes accounting above are complete. Preserve the capture-request destruction, resource cleanup, large/off-screen shape, and malformed-property coverage recorded above.

**Acceptance:** documented reproductions become regression tests when feasible; ordinary desktop activity does not crash or leave invisible/stale windows; repeated lifecycle changes do not grow server resources without bound. Maintain a compatibility matrix with evidence instead of a blanket “supported” label.

## Next: measure and reduce rendering work

This is the current milestone. The beta showed where the cost was: Compust repainted the whole screen for every damage event, asked the server for the full window tree on every stacking-related event, and repeated the blur for every translucent window. Steps 2 and 3 removed the first two, and step 4 keeps each blur until something beneath it changes. On the recorded machines a 60-updates-per-second window costs Compust under 2% of a core and the X server 3–9%, and the pyramid blur adds between a tenth of a point and one point to the server. The [first benchmark records](DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03) compare Compust with picom on one machine; no record covers a 4K screen, many windows, or a slow GPU.

The milestone has four steps, in order. Steps 3 and 4 start only if step 1 shows that they matter.

| Step | Status | Required result |
| --- | --- | --- |
| 1. Benchmark scenes and picom comparison | Recorded on the RX 9060 XT desktop; the Intel/Xorg laptop is no longer available | Fixed scenes run under Compust and under an identified picom version and backend on both recorded machines, with raw records. |
| 2. Event-path round trips | Done; a regression counts the requests | Window events no longer cost one tree query each; a test counts the requests. |
| 3. Region-based repaint | Done; region frames match full repaints pixel for pixel | Only damaged regions, expanded for blur, are repainted and presented; pixel tests cover region boundaries. |
| 4. Occlusion and blur reuse | Blur reuse done; occlusion not started | Fully covered windows are skipped and unchanged blur is reused, where the benchmark justifies it. |

### 1. Benchmark scenes and picom comparison

Extend the [desktop probe](../examples/desktop_probe.rs) with scenes that separate the costs: an idle desktop; a small window updating 60 times per second on a large screen; the existing full-screen translucent window with blur; eight overlapping translucent windows; a window moved and resized 60 times per second; and repeated open and close with the time from map to the first frame that shows the window. Record Compust and X-server CPU, memory, and Present intervals as today. Run the same scenes under picom with its version, backend, and an equivalent configuration recorded.

**Acceptance:** both recorded machines have raw records for every scene under both compositors, the runner reproduces them, and the summary states where Compust is slower as plainly as where it is faster.

**Status:** the probe's `--bench` mode and [`tools/bench.sh`](../tools/bench.sh) run the scenes under Compust and under picom v13's xrender and glx backends with [equivalent configurations](../tools/bench/). On the RX 9060 XT desktop, two runs per monitor layout [agree within 0.25 points of CPU](DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03). Every compositor held 60 frames per second except picom xrender with blur. Compust's own process used less CPU than picom glx in every scene, but the X server worked up to 2.7 points of a core harder under it than under picom glx; with both processes added, Compust was at most 0.4 points above picom glx and up to 2.1 below it. Compust showed about half of new windows one frame sooner than picom; the other half waited behind a redundant frame, and since Compust [repaints only for visible changes](DESKTOP_TESTING.md#recorded-repaint-change-2026-10-03), every new window appears one frame after its map request. The Intel/Xorg laptop is no longer available, so the comparison covers one machine; the second machine of the acceptance stays open until another one is recorded.

On that machine, moving and resizing raised Compust's own CPU from 0.3% to 1.1%, the cost step 2 targets. A 64×64 update on two monitors cost the X server 1.2 points more than under picom glx and about twice the GPU load, the cost step 3 addresses. Blur over eight windows added 0.6 points to Compust and 0.9 to the server; with step 4 it adds about 0.1 and 0.35.

### 2. Event-path round trips

`Scene::restack` queries the root's children on every map, reparent, configure, and circulate event, and `configure` queries geometry for each one. An interactive resize therefore costs several round trips per event. Track stacking from the events' own sibling fields and query the tree only when the order is unknown.

**Acceptance:** a regression counts requests through the existing test proxy and fails if a burst of configure events costs a tree query each; the stacking, destruction, and capture-race regressions still pass; step 1's move-and-resize scene shows the change.

**Status:** done. Compust mirrors the root's stacking order from structure events and queries the tree only at startup or after an event names a window the mirror does not know. A configure that keeps the size updates the position from the event alone. The [regression](../tests/cases/event_path.rs) moves and restacks windows 34 times: that cost 33 tree queries before and none now, with no geometry query. In temporary copies, disabling the stacking updates made it and the earlier stacking regression fail, and disabling the resize check made five resize regressions fail. On the RX 9060 XT desktop, the move-and-resize scene [costs Compust and the X server 0.13–0.15 points less each](DESKTOP_TESTING.md#recorded-event-path-change-2026-10-03). Because that scene resizes every frame, each event still recaptures the window, which remains the larger cost. All 66 X11 tests pass with Xorg's Xvfb 21.1.24, along with 14 unit and 6 CLI tests; XLibre's Xvfb still fails the seven that reconfigure the CRTC.

### 3. Region-based repaint

Paint and present only what changed. Damage regions must grow by the blur margin wherever a translucent window overlaps them, cover both the old and new bounds of a moved window, and include fading windows. Present needs either a region copy or a second buffer whose age is known.

**Acceptance:** pixel tests cover damage at region boundaries, under blur, across a window move, and after a monitor change; idle work does not increase; step 1's small-window scene shows the saving, and the record also shows the scenes where it does not help.

**Status:** done. Damage reports the bounding box of each window's changes, and the renderer compares what the last frame showed of every surface with the next frame: moves, resizes, recaptures, fades, departures, and shape or opacity changes add their old and new bounds, and a restack adds only where the surfaces that swapped places overlap. A blur footprint that the area touched joined it whole, repeatedly, so a blur always read a current scene; step 4 replaced that with kept backdrops. Painting is clipped to the area, Present receives it as the update region of the single buffer, and the XRender copy is clipped the same way. The [region tests](../tests/cases/regions.rs) check that a 10×10 update presents exactly its own area and leaves its neighbors intact, that a move and a restack present only the bounds and overlap involved, and that fades stay within the fading window. Blurred damage, a chain of overlapping blurs, and a monitor change match a full repaint pixel for pixel, with Present and with XRender, and an overlay exposure, such as a screen locker drawing in the overlay, is repainted. In temporary copies, removing each part of the change made at least one of these tests fail: the blur spread or its repetition, the old bounds of a move, the restack overlap, the opacity comparison, the update region, the bounding-box reports, the first full frame, and the exposure handling. On the RX 9060 XT desktop, the X server [saved 0.7 points of a core](DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04) for the small window on two monitors and 0.9 for eight translucent windows, and idle stayed idle; opening and closing measured two ticks of CPU time less, within that short scene's resolution. A full-screen translucent window and stacks of blurred windows repaint as much as before and cost the same, and the GPU load fell by about one point, not by half. All 74 X11 tests pass with Xorg's Xvfb 21.1.24, along with 21 unit and 6 CLI tests.

### 4. Occlusion and blur reuse

Skip windows fully covered by opaque, unshaped windows above them, and reuse a window's blurred background while nothing beneath it changed. Both add state that can go stale, so they are worth their complexity only if the eight-window and blur scenes in step 1 show a real cost.

**Acceptance:** each optimization has pixel tests for the case that invalidates it, and a recorded scene where it saves work.

**Status:** blur reuse is done; occlusion is not started. With region repaint alone, blur over eight overlapping windows added 0.6–0.7 points to Compust and 1.2–1.9 to the X server on the RX 9060 XT desktop, [the largest remaining cost](DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04) of that scene, because a change in the top window blurred every window beneath it again; a translucent terminal under blur did the same on each keystroke. Now every change records the lowest layer of the stack it alters, and each blurred window keeps its blurred backdrop. It blurs again only when a change beneath it reaches its footprint or the kept backdrop no longer matches its bounds; any other repaint copies the kept backdrop.

The [backdrop tests](../tests/cases/backdrops.rs) make each kind of change inside, above, beside, and beneath a blurred window, including restacks, an unmap, and moves of the window and of one beneath it, and compare every frame with one from a fresh renderer, with Present and with XRender. Changes inside or above the window present only their own area, a blurred window fading in and out blurs once, and backdrop buffers do not leak as windows map, resize, and close. In temporary copies, X11 tests failed without the re-blur for changes beneath, without the scene changes of restacks, departures, or former neighbors, with no kept backdrops, with a re-blur for a window's own changes, and with backdrops never dropped. One rule, that a window blurred again changes the scene of the windows above it, failed only its unit test: in the scenes tried, the footprint margin kept its effect from reaching the pixels above.

On the RX 9060 XT desktop, the eight-window scene with blur [cost Compust 0.50% instead of 1.02%](DESKTOP_TESTING.md#recorded-blur-reuse-2026-10-04) and the X server 0.8–1.2 points less; blur now adds about 0.1 points to Compust and 0.35 to the server over the same scene without blur. A full-screen translucent window with blur cost both 0.13–0.18 points less. The kept backdrops are server pixmaps, one per blurred window: Compust's pixmaps grew by 15 MB in the eight-window scene and by 25 MB in the full-screen one. All 80 X11 tests pass with Xorg's Xvfb 21.1.24, along with 23 unit and 6 CLI tests.

Occlusion remains open. A translucent window entirely under an opaque one is still painted and blurred where the repaint area reaches it, and keeps a backdrop, as the desktop's Alacritty window did under the benchmark's backdrop. No record yet isolates that cost.

Step 3 kept the single Present buffer: outside the update region its contents already match the screen. Multiple presentation buffers with explicit ownership remain an open evaluation for latency. Input-to-display latency needs measuring equipment this project does not have; do not report it from software timings.

## Rendering backend and protocol expansion

Evaluate an EGL/OpenGL or Vulkan backend once the import and synchronization path is specified. Modern X11 integration may involve DRI3 DMA-BUF import, modifier negotiation, Present scheduling, and explicit synchronization. Each requires real implementation and driver testing; version probes do not count.

Color management, HDR, VRR, XLibre-specific extensions, and per-output scheduling require protocol and hardware investigations before support can be promised. Native Wayland is outside this roadmap.

**Acceptance:** a backend can import real application content without routine CPU readback, handles buffer lifetime correctly, recovers from supported failure modes, and has documented driver coverage and performance results.

## Everyday usability

Beta use showed that changing `fade_ms` required restarting the compositor and that a configuration was read only when `--config` named it; [configuration discovery and reload](#everyday-usability-configuration-discovery-and-reload) now address both. Add clearer troubleshooting and distribution packaging. The Window Animations milestone below expands the existing movement/scale proposal and introduces the first typed per-window rules. Reuse that rule model for later effects. Consider shadows and rounded corners with proper shape and damage semantics.

**Acceptance:** behavior is configurable, documented in both languages, testable, and does not silently claim compatibility with picom's configuration or scripting language.

## Window Animations: planned

**Goal:** generalize the implemented fade into a per-window animation system for opening and closing: fade (opacity), pop (scale plus opacity), and slide (translation), with configurable easing and per-window rules. This expands the animation and rules work above. It is a proposed follow-up to the first beta, with no assigned release or date; the four beta gates remain unchanged.

### Starting point

| Area | Current implementation and consequence for this milestone |
| --- | --- |
| Fade | [animation.rs](../src/animation.rs) stores `from`, `to`, monotonic `started`, and `duration` in `Fade`. Integer smoothstep samples a `u16` opacity; close/reopen begin at the sampled value. Fade is implemented, including zero-duration endpoints and interrupted animations. Preserve this behavior. |
| Capture and close | [surface.rs](../src/surface.rs) uses `CompositeNameWindowPixmap` and an XRender `Picture`, not a GPU texture import. `Surface::close` marks the surface unmapped and retains its picture, named pixmap, geometry, shape, and Damage object. [scene.rs](../src/scene.rs) removes it after the fade finishes; `Surface`/`Picture` destructors release Damage, picture, and owned pixmap. A named pixmap already keeps the close contents alive after the original window disappears. |
| Remap and configure | `Scene::add` captures fresh mapped contents before replacing a closing surface, preserving its sampled fade and stacking. [events.rs](../src/events.rs) maps root children, closes on unmap/destroy, and handles reparenting. Configure updates position immediately; size/border changes recapture the pixmap while preserving the fade. Generalize those transfers to the complete animation state. |
| Render path | [paint.rs](../src/renderer/paint.rs) multiplies fade, window, and global opacity through an A8 mask and composites at the real window geometry. Shape and [blur](../src/renderer/blur.rs) also use that geometry. There is no shader, vertex buffer, model-matrix path, shadow, or rounded-corner implementation. |
| Scheduling and damage | [compositor.rs](../src/compositor.rs) keeps painting while any fade is active, requests a final repaint, and then polls without continuous repainting. `max_fps` limits work; Present requires both completion and idle before reusing its single buffer. Damage is acknowledged and marks the whole scene dirty. There is no partial repaint or buffer-age tracking. |
| Configuration and metadata | [config.rs](../src/config.rs) accepts strict TOML, with `fade_ms = 180` for both directions. Unknown fields are rejected; SIGUSR1 reloads the file, but there is no rules engine. Client discovery through `WM_STATE` exists, but animation matching by class, title, and window type does not. Root property events are selected, but `_NET_CURRENT_DESKTOP` is not handled. |
| Outputs and exclusions | RandR currently triggers root-buffer recreation; there is no per-output geometry cache. Override-redirect tooltips are not excluded from fade by type. Fullscreen unredirection is not implemented, so fullscreen state must not be confused with an actually unredirected surface. |

### Scope and preparation

1. Replace `Fade` with an `Anim` state containing kind, start time, duration, easing, open/close direction, and sampled start/target transforms. Sample once per frame into `Transform { opacity, scale, offset }`. Retarget every component from its current value, including remap during close; do not restart from an endpoint or assume value continuity also preserves velocity.
2. Apply a center-based model transform only when rendering. For local point `p`, window origin `o`, and center `c` including the border, use `p_out = o + c + scale * (p - c) + offset`. Keep actual X window geometry and input regions unchanged. Prototype this on XRender with `SetPictureTransform` and transformed destination bounds/clips: its sampling matrix maps destination coordinates back to the source, so derive the inverse and account for Composite source/destination origins. Verify filtering, identity reset, and checked fixed-point conversion before adding effects. A GPU backend is not a prerequisite. See the [Render protocol](https://xorg.freedesktop.org/archive/current/doc/renderproto/renderproto.txt).
3. Pop opens from approximately `scale = 0.85`, opacity zero, to identity and full animation opacity; closing targets the small transparent state. Offer `ease_out_cubic` or `ease_out_back` for opening and `ease_in_cubic` for closing. Keep legacy `smoothstep`; add `linear` and the cubic in/out and back-out curves as pure calculations. Clamp opacity, keep scale positive/invertible, and include back-curve overshoot in painted bounds. A spring model is optional future work.
4. Slide opens from the nearest edge of the window's current output and closes toward the selected edge; allow top/right/bottom/left overrides. Select the active RandR output with the greatest window intersection, use a deterministic tie-break, and freeze the selection for a closing snapshot. Handle negative origins, overlapping outputs, spanning windows, and output changes explicitly. If output geometry cannot be established, use fade rather than treating a multi-monitor root as one monitor. Dropdown menus can use a short displacement, proposed at 24 pixels, instead of a full edge traversal.
5. Before implementation, confirm retained-resource ownership on unmap/destroy, failed recapture, remap, and shutdown against the existing [fade](../tests/cases/fades.rs), [capture-race](../tests/cases/capture_races.rs), and [resource](../tests/cases/resources.rs) coverage. Preserve the last successful capture and its metadata until completion; keep destroyed snapshots in their established stacking position. Reuse owned pictures and masks instead of naming/copying pixmaps on every animation frame.
6. Region repaint compares each surface's shown state between frames. Make the sampled transform part of that state and the surface's bounds its transformed bounds, including back-curve overshoot, so every animation frame repaints the union of the previous and current transformed bounds, widened for blur as today and for any future shadow. Preserve Present buffer ownership, direct-XRender fallback, frame limiting, and return to the existing idle poll, including the final cleanup frame. Confirm these paths for several simultaneous animations and zero duration.

### Proposed configuration and eligibility

Extend the existing TOML format with `[animations]` and ordered `[[animation_rules]]`. **These fields are a proposal and are not accepted by the current binary.** Keep `compust.example.toml` valid until implementation. The owner must confirm the public schema before coding, following the [feature proposal template](../.github/ISSUE_TEMPLATE/feature.yml).

- Global fields: `kind` (`none`, `fade`, `pop`, `slide`), separate `open_ms`/`close_ms`, `open_easing`/`close_easing`, `pop_scale` (default `0.85`), `slide_direction` (`nearest` by default, or `top`, `right`, `bottom`, `left`), optional `slide_offset_px` (absent means edge traversal), and `suppress_workspace_switch` (proposed default `true`).
- Compatibility defaults: fade, smoothstep in both directions, and durations inherited from `fade_ms` (180 ms when absent). Explicit new duration fields override that legacy value per direction; zero completes immediately. Proposed opt-in pop/slide examples use 220 ms open and 150 ms close. The refactor preserves eligible-window fade output exactly; new exclusions and workspace suppression are intentional eligibility changes.
- Rules match cached client `WM_CLASS` resource class, `_NET_WM_WINDOW_TYPE`, and name (`_NET_WM_NAME`, falling back to `WM_NAME`). Use the existing client/frame association, validate property types/lengths, and preserve close metadata after destruction. Proposed matching is case-sensitive exact text, all supplied selectors must match, and the first matching rule overrides only specified global fields. Missing/malformed properties do not satisfy a selector. Refresh metadata for future transitions without restarting an active animation merely because a title changes.
- Exclude override-redirect tooltips and any surface actually outside compositing; rules may exclude other windows with `kind = "none"`. These hard exclusions take precedence over enabling rules. Override-redirect dropdown menus remain eligible for explicit short-slide rules. Do not add fullscreen unredirection in this milestone; preserve the exclusion contract if that feature is introduced later.
- Reject unknown keys/kinds/curves, invalid durations, non-finite or non-positive scales, and out-of-range offsets before connecting to X11. Rules reload with the rest of the configuration and, like `fade_ms`, apply to transitions that start afterward.

Proposed opt-in example, not a current configuration:

```toml
[animations]
kind = "pop"
open_ms = 220
close_ms = 150
open_easing = "ease_out_cubic"
close_easing = "ease_in_cubic"
pop_scale = 0.85
suppress_workspace_switch = true

[[animation_rules]]
window_type = "dropdown_menu"
kind = "slide"
slide_direction = "top"
slide_offset_px = 24

[[animation_rules]]
wm_class = "ExampleApp"
name = "No animation"
kind = "none"
```

### Interaction, effects, and risks

- **Input:** X routes clicks to real geometry, not the transformed image. Keep the proposed open duration within 200–250 ms and close near 150 ms, expose disabling rules, and document the mismatch. This milestone does not move windows or synthesize input.
- **Workspaces:** observe changes to the root `_NET_CURRENT_DESKTOP` property and suppress the associated per-window open/close animations. Coalesce lifecycle decisions around frame/event batches, cancel or settle affected animations on a desktop change, and bound the suppression interval so later ordinary opens still animate. Test property notifications both before and after map/unmap events, across the 512-event batch boundary, and with rapid successive switches. Record actual Xmonad event ordering before fixing the interval; a count of unmaps alone is not reliable detection. WMs that do not publish a desktop change require an explicit disabling policy and must not be advertised as covered. The [EWMH property](https://specifications.freedesktop.org/wm/1.5/ar01s03.html) provides the signal, not a generic transaction boundary.
- **Tiling and configure:** neighbors still resize/move immediately when the WM arranges a new window. Only the entering/leaving window animates. Configure must preserve its ongoing animation when replacing a picture and must not launch animations for ordinary movement/resizing.
- **Effects:** transform Shape clipping, borders, per-pixel alpha, opacity masks, and blur coverage with the same geometry. Blur must sample the scene behind the animated destination, not move an old blurred background patch. Shadows and rounded corners do not exist today; adding them is separate work, and any future effect must consume the same transform and bounds. Test overshoot, empty/disjoint shapes, large/off-screen windows, and cleanup of reusable picture transform state.
- **Outputs and cost:** nearest-edge calculations require output geometry, not root dimensions. Resolve topology changes without accessing destroyed windows or jumping to stale output coordinates. Full-screen blur can dominate X-server CPU during simultaneous animations; measure Compust and the server separately, avoid allocations/round trips per frame, and qualify frame pacing only for recorded workloads and hardware.

### Verification and acceptance

Keep pure sampling tests separate from real-server pixel/protocol tests. Follow CONTRIBUTING's event deadlines and resource-lifetime rules; do not hide new X11 errors or weaken existing tests. Use the current Xvfb fixtures, the isolated [Xmonad runner](../tools/desktop-check.sh), and recorded physical Xorg/XLibre sessions where required.

- [ ] Legacy fade has identical smoothstep values, opacity multiplication, duration behavior, zero-duration endpoints, and final repaint for the same eligible event sequence; existing regressions still pass.
- [ ] Pop and slide open and close normal windows correctly, including center scaling, direction overrides, and short dropdown slides.
- [ ] Unmap/destroy close animations keep the last captured contents without black/blank frames; resources are released after completion.
- [ ] Open-then-close and remap-during-close start from the current opacity, scale, and offset without a jump; recapture failure preserves the previous snapshot and ordering.
- [ ] Xmonad workspace switches produce no per-window animation storm or lingering close snapshots; normal opens resume afterward, including rapid repeated switches.
- [ ] Excluded tooltips and user-excluded windows never animate. The unredirected-surface guard is covered without claiming fullscreen unredirection exists.
- [ ] Per-window rules override global defaults with documented precedence; client/frame metadata, missing/malformed values, legacy config, and invalid config are covered.
- [ ] Shape, alpha, borders, and blur remain attached and correctly clipped throughout transforms. Future shadows/rounded corners must meet this same check when implemented.
- [ ] Mixed-resolution outputs, negative origins, spanning windows, nearest-edge selection, and output changes during opening/closing are correct on documented monitor layouts.
- [ ] At least eight simultaneous opens/closes meet the declared frame budget without animation-induced frame drops on the qualified setup; record refresh rate, `max_fps`, blur setting, CPU, Present intervals, and the no-animation comparison. Test Present and direct-XRender paths; do not infer tear-free scanout from software timings.
- [ ] After animations finish, repainting stops and CPU returns to the recorded idle baseline; no busy loop, delayed final frame, or permanently active animation remains.
- [ ] After warmup and at least 1,000 open/close cycles, matching settled scenes show no growth in XRes resource counts or full owned-pixmap bytes; record process/server memory trends and test interrupted cycles and shutdown.
- [ ] Formatting, strict Clippy, full regressions, release build, and bilingual documentation pass before advertising the milestone as implemented.

### Ordered tasks

Each item is one reviewable issue. Its fields follow the feature template: **Problem or use case**, **Proposed behavior**, and **How to verify it**. The last field is the done criterion.

1. **Confirm lifetime, scheduling, and configuration contracts.** **Problem or use case:** transforms add state to already working close/remap paths. **Proposed behavior:** record ownership and event-order traces in `surface.rs`, `scene.rs`, `events.rs`, and `compositor.rs`; approve the TOML schema/defaults, full-repaint strategy, output/clipping policy, and workspace suppression boundary. **How to verify it:** every preparation item has a concrete decision and a mapped existing or proposed regression; resolve public-schema questions before implementation.
2. **Generalize fade without changing its output.** **Problem or use case:** `Fade` samples opacity only. **Proposed behavior:** add typed `Anim`, direction/easing, and transform sampling in `animation.rs`; migrate surface creation, closing, remap, resize replacement, and scene cleanup with identity scale/offset. **How to verify it:** exact old fade samples and all interrupted/zero-duration pixel and lifetime regressions pass; failed recapture does not discard a closing surface.
3. **Add configuration, metadata, rules, and exclusions.** **Problem or use case:** global `fade_ms` cannot express per-window policy. **Proposed behavior:** extend `config.rs`, `atoms.rs`, and client/surface metadata with strict TOML resolution, first-match rules, separate open/close settings, and exclusions; retain legacy defaults. **How to verify it:** parsing/precedence cases cover valid, absent, malformed, and conflicting data; frame/client and destroyed-client cases select the intended policy. Expose each new animation kind only when its rendering task is ready.
4. **Introduce the XRender transform path.** **Problem or use case:** painting/blur/clips use untransformed geometry. **Proposed behavior:** add center-based affine sampling, inverse picture transforms, destination bounds, consistent effect coverage, and reusable transform/filter state in `picture.rs` and `renderer/{paint,blur}.rs`. **How to verify it:** identity matches existing pixels; controlled scale/offset samples preserve centers, shapes, borders, transparency, blur, and extreme off-screen clipping without per-frame resource growth.
5. **Implement pop and selectable easing.** **Problem or use case:** the generic path needs a complete scale/opacity effect. **Proposed behavior:** add the 0.85-to-1 pop endpoints, cubic/back/linear sampling, bounded opacity and scale, and configurable open/close curves. **How to verify it:** pure endpoint/overshoot/interruption cases and real-server open, unmap, destroy, and remap scenes pass with preserved last contents.
6. **Implement output-aware slide and menu offsets.** **Problem or use case:** root-wide edges are wrong on multiple monitors. **Proposed behavior:** cache negotiated RandR monitor/CRTC geometry, refresh on output/CRTC changes, select the window's output, and add nearest/explicit directions, dropdown offsets, and documented fallback. **How to verify it:** deterministic geometry cases plus pixel captures cover different resolutions, negative origins, ties, spanning windows, hotplug/resize during animation, and missing RandR information.
7. **Suppress workspace-driven animations.** **Problem or use case:** a workspace transition can resemble many independent window openings/closings. **Proposed behavior:** add root desktop tracking and bounded lifecycle classification in `atoms.rs`, `events.rs`, and `compositor.rs`; settle affected snapshots and preserve genuine later transitions. **How to verify it:** reordered/batched protocol scenarios and an actual Xmonad session pass switch, rapid-switch, interrupted-animation, and subsequent ordinary-open checks; record unsupported WM behavior.
8. **Qualify performance, cleanup, and documentation.** **Problem or use case:** visible correctness alone cannot establish idle behavior or resource stability. **Proposed behavior:** extend `tests/cases/`, the XRes helpers, and the desktop probe for concurrent transforms, repeated lifecycle changes, and measured pacing; update the README, architecture, valid example TOML, and both roadmaps when implemented. **How to verify it:** every acceptance checkbox has evidence for the exact commit and declared environment; proposed-only features remain labeled planned until their checks pass.

### Decisions and later work

Owner confirmation is still needed for the proposed `[animations]`/`[[animation_rules]]` public schema, first-match exact-string semantics, and the opt-in 220/150 ms presets. The plan preserves the 180 ms legacy fade default and proposes enabling workspace suppression. No new release date or beta gate is assigned.

Animating move/resize geometry of existing windows and whole-workspace slide transitions are out of scope and remain future roadmap items. A spring model, shadows, rounded corners, fullscreen unredirection, and partial repaint remain separate work.

There are no delivery dates yet. Open an issue to discuss a bounded change or contribute an observed failure; avoid starting several overlapping backend designs before agreeing on the requirements.
