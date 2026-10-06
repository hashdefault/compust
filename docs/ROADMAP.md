# Roadmap

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

The goal is a minimal Rust compositor that becomes a practical choice for Xorg and XLibre users; the [1.0 milestone](#10-stable-release) defines what that takes. Progress is measured through Compust's own correctness, reliability, latency, and resource records. Rewriting an existing feature in Rust is not, by itself, evidence of better performance.

## Status and priorities (2026-10-05)

The latest release is [0.3.0-beta.2](#fifth-beta-prerelease), a GitHub prerelease with Open Build Service packages for Debian, Ubuntu, Linux Mint, Fedora, and openSUSE Tumbleweed. This section sums up what is done and orders what comes next; the sections below hold the records.

### Done

| Area | State | Records |
| --- | --- | --- |
| Foundation and first beta | XRender composition, fades, transparency, blur, shapes, and stacking; the four beta gates met for their scope | [0.1 foundation](#01-foundation-implemented), [first beta](#first-beta-four-steps) |
| Everyday use | Configuration discovery, reload on SIGUSR1 and when the file is saved, per-window rules, and weighted blur | [Everyday usability](#everyday-usability) |
| Rendering | Region repaint, blur reuse, occlusion, and the opt-in GPU renderer | [Rendering work](#rendering-work-four-steps-done-on-one-desktop), [GPU backend](#rendering-backend-and-protocol-expansion) |
| 1.0 step 1 | Shadows, fullscreen unredirection, and rules by focus, released in 0.3.0-beta.2 | [Step 1](#1-features-picom-users-rely-on) |
| Rounded corners | Tasks 1 to 5: corners, rounded shadows, and blur at the arcs in both renderers, released in 0.3.0-beta.2 | [Rounded Corners](#rounded-corners-implemented) |
| Hardware | AMD on XLibre with two desktops and five window managers, AMD on Xorg with i3 and both renderers, and Intel on Xorg with four window managers, hotplug, and suspend | [Compatibility matrix](#compatibility-matrix) |
| Distribution | Reproducible release archives with checksums, and Open Build Service packages built and tested from the release tag | [Fifth beta](#fifth-beta-prerelease), [packaging](../packaging/obs/README.md) |
| Planned | Active and Inactive Opacity, its schema decided; Window Animations | [Active and Inactive Opacity](#active-and-inactive-opacity-planned), [Window Animations](#window-animations-planned) |

### Priorities

In order, each with the reason it comes before the next. Items 1 to 6 lie on the [path to 1.0](#release-path).

1. **Find the stale frame after a monitor change.** The X11 test `regions::monitor_changes_repaint_the_whole_screen` failed in 4 of 12 full parallel runs on the Radeon Vega desktop on 2026-10-05, as often before the rounded-corner work as after it: after a monitor change, 4,800 pixels differed from a full repaint, from (0, 180) on. It passed 20 runs in a row alone, in CI, and in every Open Build Service build. 1.0 promises that ordinary activity never leaves stale pixels, and a test that fails at random also hides new failures. Reproduce it under load, find the ordering it depends on, and fix it with a regression that fails without the fix.
2. **Bring 0.3.0-beta.2 to outside testers.** [Step 5](#5-outside-testing) needs three testers other than the maintainer and then two quiet weeks, the longest wait on the path to 1.0. Announce the beta where X11 users gather, pointing to the beta guide and the issue templates.
3. **Record more hardware.** [Step 2](#2-hardware-and-desktops) still lacks NVIDIA's driver on Xorg, Intel with the current code, more window managers on Xorg, physical hotplug and suspend and resume on Xorg, and actual applications: a browser and its menus, a terminal, mpv, a game, a client-side-decorated GTK window, and an Electron application. Those sessions also cover what the rounded corners' last task asks: corners under a window manager and with applications. A [live qtile session](DESKTOP_TESTING.md#recorded-qtile-session-on-the-intel-laptop-2026-10-05) on the Intel laptop has since recorded corners, shadows, and blur under one window manager in both renderers, with synthetic windows and without the probe. Testers' reports from item 2 can supply part of it.
4. **Start the endurance runs.** [Step 4](#4-endurance) has not started. Write the automated run of 10,000 window cycles with XRes checks, and count the week of daily use on the Radeon Vega desktop, which already runs Compust as its compositor.
5. **Prepare distribution and review the configuration.** [Step 6](#6-distribution-and-documentation) still needs an Arch User Repository package, manual pages, a picom migration guide, and a troubleshooting guide. Before 1.0.0-rc.1, review every setting once, `corner_radius` included, since 1.0 freezes them.
6. **Measure the release candidates.** [Step 3](#3-independent-performance-measurements) runs the benchmark scenes on the candidates of item 5 in each environment item 3 qualifies, including the cost of shadows and rounded corners.
7. **Active and Inactive Opacity, tasks 2 to 6.** Rules by focus already dim inactive windows, so this milestone is recommended after 1.0.0-rc.1, keeping 1.0's settings as they are. If its settings are to be part of 1.0, they must land before the configuration review of item 5.
8. **Window Animations**, after 1.0.

## 0.1 foundation: implemented

The repository contains an executable compositor with XRender composition, fades, alpha transparency, blur, shape clipping, application redraw tracking, stacking, window resize handling, wallpaper properties, and optional Present copy scheduling. The test suite checks real server behavior on Xvfb. Documentation and contribution paths are available in English and pt-BR.

This milestone establishes a base for experiments. It does not establish desktop-wide compatibility or hardware performance.

## First beta: four steps

The first beta uses the XRender backend and declares support only for environments with recorded test evidence. It is intended for controlled community testing. Version 0.2.0-beta.1 was that beta, 0.2.0-beta.2 and 0.2.0-beta.3 follow it, and [0.3.0-beta.1](#fourth-beta-prerelease) starts the path to 1.0; the [beta guide](BETA.md) lists the current declared scope.

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

The Xvfb scenarios below pass. A native XLibre session on AMD hardware also passed physical unplugging and reconnection of both connectors, plus two-monitor mode and layout changes, in both presentation modes. [HDMI-1 at 60 Hz and DP-2 at 180 Hz](DESKTOP_TESTING.md#recorded-mixed-refresh-monitors-2026-10-04) are now recorded on the RX 9060 XT in XRender and GL. Other drivers and servers and more than two monitors need their own records; disabling a virtual CRTC does not establish physical behavior.

### 3. Real desktops

Run documented scenarios with real window managers on Xorg and XLibre. Record the server, window manager, GPU/driver, configuration, and exact commit. Measure idle and active CPU, memory, and frame pacing; fix failures in the environments proposed for beta support.

**Acceptance:** publish a compatibility matrix with evidence for each advertised environment, a reproducible measurement baseline, and remaining limitations. Untested driver/server combinations remain unqualified.

The [desktop qualification guide](DESKTOP_TESTING.md) documents the isolated runner, measurements, and hardware procedure. The recorded Xephyr/Xmonad runs start this step; they do not close its driver and physical-display requirements. A dedicated AMD/XLibre session now runs the probe's scenarios, wallpaper change, fades, and translucent blur on hardware. Actual applications, decorated or reparenting window managers, Xorg on hardware, and other drivers remain open.

### 4. Beta distribution

Publish a versioned beta prerelease with build or binary installation instructions, configuration examples, startup/shutdown guidance, checksums for distributed artifacts, known limitations, and a bug-report template including diagnostics and reproduction steps.

**Acceptance:** a tester can install and run the exact release, return to their previous compositor, and report a failure from the supplied instructions. CI passes for the release commit and the previous three gates are satisfied for its declared support scope.

Version 0.2.0-beta.1 completed these four acceptance gates for its declared scope. GPU backend expansion and advanced effects can follow the first beta. The [rendering milestone](#rendering-work-four-steps-done-on-one-desktop) followed it, and [1.0](#10-stable-release) is the current one; widening the beta's hardware coverage continues beside them as reports arrive.

## Progress and verification

### 1.0 step 1: features recorded on AMD/XLibre

`desktop_probe --features` now checks a fixed scene without a window manager: every shadow-scene pixel against an independent profile, focus changes controlled through `_NET_ACTIVE_WINDOW`, and fullscreen suspension and resume with the same drawing load and separate CPU records. [`tools/features-check.sh`](../tools/features-check.sh) runs XRender and GL, compares their shadow and focus images, and rejects a GL run that falls back to XRender. [`tools/hardware-session.sh --features`](DESKTOP_TESTING.md#check-shadows-focus-and-fullscreen-suspension) starts this procedure in a dedicated hardware session. The probe passed on Xvfb, including deliberate bad configurations, then passed with both painters on the [RX 9060 XT / XLibre hardware session](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04). Every shadow-scene pixel was within one RGB level of the independent reference, the focus images were identical between painters, and suspension stopped Present and overlay Damage during 600 redraws per phase. Compust accumulated no CPU ticks while suspended over ten seconds, compared with 0.3% of one core while composed in XRender and 0.6% in GL. Popup resume, fresh recapture, and shutdown passed. This record uses one display and synthetic windows without a window manager; actual applications and other hardware remain to be qualified.

### 1.0 step 1: rules by focus

Rules gain a `focused` selector: `true` chooses the window that the window manager reports as active in the root's `_NET_ACTIVE_WINDOW`, and `false` every other window, so that a rule can dim inactive windows as picom's `inactive-opacity` does. The property names the client, so a frame is focused when its client is. A change to it costs one read and resolves the rules again for the windows that lost and gained focus. Under a window manager that does not set the property, every window counts as focused, so a rule for unfocused windows changes nothing instead of dimming the whole desktop. On Xorg's Xvfb, with a terminal open, Openbox 3.6.1, i3 4.25.1, bspwm 0.9.12, and dwm 6.8 set the property, and Xmonad 0.18.1 set it with `XMonad.Hooks.EwmhDesktops` and not with its default configuration.

Six X11 tests in [focus.rs](../tests/cases/focus.rs) cover the rule following the active window, with no active window, without the property, and with one of the wrong type; a window mapped while another is active; windows open before Compust starts; focus kept through a title change, a resize, a reload, and a suspension of compositing; focus combined with a type selector; a client in a frame, also one found after the frame was shown; and one property read per focus change. Each of 10 changes to the implementation, made one at a time and reverted, failed at least one of them. With this, the three features of step 1 are implemented; their fixed fixtures now have a [hardware record with both painters](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04), with focus controlled by the probe. With Xorg's Xvfb, all 120 X11 tests, 35 unit tests, and 6 CLI tests pass, as do the GPU crate's 9 tests, with formatting and strict Clippy.

### 1.0 step 1: fullscreen unredirection

With `unredirect_fullscreen = true`, compositing is suspended while the topmost window shown is opaque, unshaped, and covers the whole screen: Compust stops redirecting windows, which then draw to the screen directly, and unmaps its overlay. It resumes when a viewable window maps, when the cover moves, changes size, shape, or opacity, is restacked beneath another window, or closes, and when the setting is reloaded off. The plan had allowed opaque windows above the cover. Anything above it now resumes compositing, so that a menu or a notification keeps its shadow and fade, and the rule stays that one window hides everything else. A window that receives no drawing changes nothing when it maps above the cover. The setting is off by default.

Resuming redirects every window into a new pixmap that starts with what the window shows, so each mapped surface is captured again, and a window that closed while compositing was suspended goes without a fade, because Compust holds only its image from before. While suspended, Compust keeps its buffers and the surfaces' old pixmaps; freeing them is left for later.

Eight X11 tests in [unredirect.rs](../tests/cases/unredirect.rs) cover a cover that maps and unmaps, with Present and with XRender; what a window drew while suspended, shown once a window maps above it; every change that uncovers the screen, each followed by the right pixels; windows with an alpha channel, one pixel short of the screen, and input-only; a cover that fades in; a cover destroyed while suspended; and resource counts over five cycles. Each of 14 changes to the implementation, made one at a time and reverted, failed at least one of them. These regressions ran on Xvfb. The [hardware probe](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04) then recorded suspension, CPU use, and fresh recapture with both painters on one AMD/XLibre display; how actual fullscreen applications enter and leave it remains to be recorded. With Xorg's Xvfb, all 114 X11 tests, 35 unit tests, and 6 CLI tests pass, as do the GPU crate's 9 tests, with formatting and strict Clippy.

### 1.0 step 1: shadows

Windows cast a shadow once `shadow_radius` is above zero: a black copy of the window's rectangle, moved by `shadow_offset_x` and `shadow_offset_y`, blurred over the radius, and as dark as `shadow_opacity` times the window's opacity. Shadows are off by default, so an upgrade changes no desktop. A window casts one when its type is normal, dialog, utility, splash, or toolbar. A window that names no type and that the window manager leaves alone casts none, nor does one whose `_GTK_FRAME_EXTENTS` declares margins for a shadow of its own, and `shadow` in a rule decides either way. Shaped windows never cast one: a shadow that follows a shape waits for rounded corners, after 1.0.

Blurring a rectangle separates into one profile along each axis, so each painter keeps two strips per window and multiplies them at every pixel, which costs XRender two small composites and the GPU painter one draw. The XRender strips have room to spare, so a resize uploads new profiles without allocating. Three box filters computed with integers give the profiles, the same in both painters. A shadow lies around its window and never beneath it, is painted after the window's own blur has read the scene, and is part of what a frame showed of the window: moves, fades, and reloads repaint both, while the window's own content leaves the shadow alone. A window hidden whole behind an opaque one still shows the part of its shadow that reaches past it.

Twelve X11 tests in [shadows.rs](../tests/cases/shadows.rs) check the shadow's extent, offset, and profile pixel by pixel, also where it starts off the screen; that a translucent window stays as bright as without shadows; which types, margins, shapes, and rules cast one, and that each of those changes an open window's shadow; fades, reloads, and resizes without leaked buffers, and without a new buffer for a small step; shadows around a hidden window and a hidden blur; and, after every kind of change to a scene with a blurred window, a frame equal to one from a fresh renderer, with Present and with XRender. Each of 21 changes to the implementation, made one at a time and reverted, failed at least one of them. The GPU crate's test draws a shadow from two profiles on Mesa's software device and, where a render node exists, on that GPU, where it passed with radeonsi. The [hardware probe](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04) now checks the GPU painter and XRender under glamor against an independent shadow reference and compares their whole-screen images. These fixtures passed on the RX 9060 XT within one RGB level; shadow performance and other hardware remain to be recorded. With Xorg's Xvfb, all 106 X11 tests, 35 unit tests, and 6 CLI tests pass, as do the GPU crate's 9 tests, with formatting and strict Clippy.

### Everyday usability: configuration discovery and reload

Beta use showed that changing `fade_ms` required a restart and that a configuration was read only when `--config` named it. Without `--config`, Compust now reads the first `compust/compust.toml` in `$XDG_CONFIG_HOME` (default `~/.config`), then in `$XDG_CONFIG_DIRS` (default `/etc/xdg`), and `--check-config` names the file it found. SIGUSR1 reloads the configuration a restart would read. A file that cannot be read or is invalid is logged, and the running configuration stays. A reloaded `fade_ms` applies to every transition that starts afterward, and a blur or vsync change replaces the renderer once Present has released its buffer. A rejected Present submission now marks the extension unavailable for the session instead of turning off `vsync`, so a reload cannot bring back a path the server refused.

Seven X11 tests in [reload.rs](../tests/cases/reload.rs) cover an opacity reload; a reloaded fade duration closing a window opened before it; rejected reloads with an out-of-range value, a syntax error, and a removed file; blur toggled sixteen times in each presentation mode with unchanged XRes counts and pixmap bytes; switching between Present and direct copying; and a reload after a Present rejection. Each failed when its part of the behavior was disabled in a temporary copy. Unit and CLI tests cover the search order, a dangling link, and `--config` taking precedence over discovery.

The suite ran with XLibre's Xvfb 25.1.9, because Xorg's conflicts with the XLibre packages on that machine. The seven existing tests that reconfigure the RandR CRTC fail there before and after this change: that Xvfb reports a 1280×1024 CRTC on a 320×240 screen and rejects `SetCrtcConfig` with `BadValue`. The other 58 X11 tests, 12 unit tests, and 6 CLI tests pass, with formatting and strict Clippy; CI uses Xorg's Xvfb. A [live session](DESKTOP_TESTING.md#recorded-reload-session-2026-10-03) on a third machine, XLibre with a Radeon RX 9060 XT under dwm, reloaded blur, fades, and vsync and rejected an invalid file across one and two monitors. Every Present interval spanned one vblank, and repeated states held identical owned pixmap bytes.

### Everyday usability: reload on save

Applying an edited configuration still took a signal. Compust now also reloads when the file is saved: [watch.rs](../src/watch.rs) watches, through inotify, the directory of every file the search would read and of the file that a symbolic link among them points to, and reloads once the files have stayed unchanged for 100 ms, so a save in several steps reloads once. SIGUSR1 still reloads, and remains the only way where inotify is unavailable and for a configuration directory created after Compust starts.

Five unit tests cover saves in place, by renaming, and by removal; other files in the same directory; the settling of a save that moves the old file aside first; links into another directory; and replacing the watched files. An X11 test in [reload.rs](../tests/cases/reload.rs) saves the configuration in place, by renaming a new file over it, and as Vim does, without a signal, and waits for each opacity to show; temporary copies that never let a change settle or that miss renames make it fail. On a private Xvfb, the release binary logged exactly one reload, about 100 ms after the last write and without a warning, for each of those saves and for a burst of three writes within 60 ms. On the Radeon Vega desktop's usual i3 4.25.1 session on Xorg 21.1.24, the installed release build of `ed2cdb0` (`cc8ad9e1…`) logged a reload within 0.6 seconds of its configuration file being written back unchanged, and the owner reports that saved edits apply in daily use; that check is not archived. All 123 X11 tests pass with Xorg's Xvfb 21.1.24, along with 45 unit and 6 CLI tests, strict Clippy, the release build, and the feature checks.

### Everyday usability: per-window rules and weighted blur

A frosted halo around Brave's right-click menus on the AMD/XLibre desktop, under picom too, came from blurring behind whole 32-bit windows: the menu's window extends 24 pixels past the menu at the sides, 12 at the top, and 36 at the bottom, holding a shadow at most 16% opaque. The blurred background now shows through each pixel as strongly as the window covers it, its alpha times its opacity, in both painters, so transparent margins stay sharp without configuration and blur fades in and out with a window. A test in [backdrops.rs](../tests/cases/backdrops.rs) maps a menu-like window whose margin must leave the background sharp.

Per-window rules choose windows by resource class, EWMH type, or title, matching exact text, and set their opacity, blur, and fade duration; each setting comes from the first matching rule that sets it. A capture reads the client's identity in one round trip, and a resize keeps it without reading it again. Changes to those properties, a client found later, and a reload resolve the rules again. A frame that loses its client, and a window already gone when its last change is read, keep their identity, so a closing window fades out with its own rules. A window without a type takes the EWMH default, `dialog` when transient and managed and `normal` otherwise, where the [animation plan](#proposed-configuration-and-eligibility) had proposed that a missing type match nothing.

Eight X11 tests in [rules.rs](../tests/cases/rules.rs) cover a class rule replacing the global opacity through a reparenting frame, before and after `WM_STATE` and after the client is destroyed; a window destroyed right after a title change; `_NET_WM_NAME` before `WM_NAME` as Latin-1 `STRING` or UTF-8; known, unknown, and absent types, with transient and override-redirect windows; blur turned off and kept on by rules, and both reversed by a reload; a slow fade through opening, closing, and reopening; opacity rules added and removed by reloads; and four resizes that read no class while a title change reads it once. Each of 19 changes to the implementation, made in temporary copies, failed at least one of them. Writing the destroyed-window test found a bug: an error from the first of the batched property reads returned early and left the other replies' errors to arrive as events, which stop the compositor; every reply is now read first. Unit tests cover validation, matching, precedence, and `WM_CLASS` parsing. With Xorg's Xvfb, all 94 X11 tests, 33 unit tests, and 6 CLI tests pass, as do the GPU crate's 7 tests on Mesa's software device, with formatting and strict Clippy.

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

### Fourth beta prerelease

Version 0.3.0-beta.1 carries everything since 0.2.0-beta.3: configuration discovery and reload, per-window rules, weighted blur, the event-path and repaint changes, region repaint, blur reuse, occlusion, and the opt-in GPU renderer. It is the first release on the [path to 1.0](#release-path). The [beta guide](BETA.md) lists the scope, and the [release notes](releases/v0.3.0-beta.1.md) list the changes.

Its declared scope covers one machine. On the RX 9060 XT desktop, [dedicated sessions](DESKTOP_TESTING.md#recorded-desktop-sessions-on-the-rx-9060-xt-2026-10-04) with the release's compositor source passed every desktop scenario under Xmonad, Openbox, i3, and bspwm in all three modes. Benchmark scenes at earlier commits ran there under dwm. The Radeon Vega desktop and the Intel laptop were recorded with the earlier painter, and the laptop is no longer available. The per-window rules have automated tests only; the weighted blur was also checked by eye on Brave's menus. The sessions used a local build, not the packaged binary.

Two [`package.sh`](../tools/package.sh) runs on the RX 9060 XT desktop produced identical archives. The packaged binary printed its version and needs no glibc symbol newer than 2.34; it links no EGL library, which the GPU renderer loads when it starts. On Xorg's Xvfb 21.1.24 it refused to start beside another compositor, exited successfully after SIGTERM, and exited with a connection error when its X server stopped.

### Fifth beta prerelease

Version 0.3.0-beta.2 carries everything since 0.3.0-beta.1: the three features of [1.0 step 1](#1-features-picom-users-rely-on), shadows, fullscreen unredirection, and rules by focus; [rounded corners](#rounded-corners-implemented); [reloads when the configuration is saved](#everyday-usability-reload-on-save); and two fixes, for a replaced buffer presented late and for the GPU renderer's overlapping clips. The [beta guide](BETA.md) lists the scope, and the [release notes](releases/v0.3.0-beta.2.md) list the changes.

Its declared scope covers one machine: the Radeon Vega desktop, now on Xorg 21.1.24. Its [desktop sessions](DESKTOP_TESTING.md#recorded-xorg-sessions-on-the-radeon-vega-desktop-2026-10-05) under i3 passed every scenario at `ed2cdb0`, before rounded corners and the GPU fix, and its [feature checks](DESKTOP_TESTING.md#recorded-rounded-corners-on-the-radeon-vega-desktop-2026-10-05) passed in both painters at `1fc0a1c`, whose compositor source differs from the release's only in its version number. The RX 9060 XT desktop's records cover 0.3.0-beta.1 and earlier commits. All of them used local builds, not the packaged binary.

Two [`package.sh`](../tools/package.sh) runs on the Radeon Vega desktop produced identical archives. The packaged binary printed its version and needs no glibc symbol newer than 2.34; it links no EGL library, which the GPU renderer loads when it starts. On Xorg's Xvfb 21.1.24 it refused to start beside another compositor, exited successfully after SIGTERM, and exited with a connection error when its X server stopped. The downloaded release archive matched `SHA256SUMS`. The [Open Build Service packages](../packaging/obs/README.md) build the tag as `0.3.0~beta.2` for Debian 13, Fedora 43 and 44, and openSUSE Tumbleweed; every build ran the full test suite, all 132 X11 tests included, and passed.

### Compatibility matrix

| Environment | Verified coverage | Evidence / limits |
| --- | --- | --- |
| Xvfb 21.1.24 on CachyOS, 320×240×24, XRender and Present 1.2 | Clients/frames, properties, rapid sequences, interrupted fades, shapes, queued destruction events, and eleven capture-request boundaries; all 47 tests pass | Recorded 2026-10-02. Capture races use `fade_ms = 0`, `blur_radius = 0`, and default vsync. Earlier fade/shape cases also use `fade_ms = 1000`, `blur_radius = 4`, or `vsync = false` as described above. Hierarchies are created directly, without real window-manager or GPU qualification. |
| Same Xvfb, one virtual output, RandR and XRes | Root shrink/restore, CRTC disable/restore, rejected Present submission, and repeated resource accounting; all 59 tests pass | Recorded 2026-10-02 (local time). Server resource counts and bytes are checked at matching rendered states; physical hotplug and multiple monitors are outside this virtual setup. |
| Xorg 21.1.11 native on Linux Mint 22.3, modesetting + i915, Intel Core i3-1005G1 (Iris Plus G1, Mesa 25.2.8), Xmonad 0.17.2, one 1366×768 panel at 60 Hz | Nested checks; panel mode change and restoration in both presentation modes; probe desktop scenarios in Present, direct, and effects modes; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-intelxorg-sessions-2026-10-03). Physical hotplug was recorded later with an [external display](DESKTOP_TESTING.md#recorded-intelxorg-hotplug-session-2026-10-03). |
| Same Intel/Xorg laptop with Openbox 3.6.1 | Nested checks, panel mode change, and probe desktop scenarios in all modes, including decorated frames, restacking, and iconify | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-openbox-sessions-2026-10-03). One [suspend and resume](DESKTOP_TESTING.md#recorded-suspend-and-resume-2026-10-03) per presentation mode also passed. |
| Same Intel/Xorg laptop with i3 4.23 | Nested checks, panel mode change, and probe desktop scenarios in all modes, including framed title bars | Recorded 2026-10-03 with synthetic windows in the default split layout. [Results and limitations](DESKTOP_TESTING.md#recorded-i3-sessions-2026-10-03). |
| Same Intel/Xorg laptop in its usual session with qtile 0.37.1, 3-pixel X borders and 10-pixel margins | Live checks of rounded corners and their borders, shadows, transparency, blur, focus changes, group switches, fullscreen, and rapid window lifecycles, in XRender and GL; CPU and RSS | Recorded 2026-10-05 with the packaged `0.3.0~beta.2+git20261005.3dfd920-1` and synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-qtile-session-on-the-intel-laptop-2026-10-05). A driver written for the session measured captures; the probe's scenarios, Present pacing, and applications were not run. RSS stayed high after leaving GL, and Present timeouts were logged once after the runs. |
| Dedicated Xorg 21.1.24 sessions on the same Radeon Vega machine (radeonsi, renoir), HDMI-1 alone at 1920×1080 60 Hz, glamor without TearFree, with i3 4.25.1 and without a window manager | Probe desktop scenarios with wallpaper change; Present, direct XRender, and effects; shadows, focus, rounded corners, and fullscreen suspension in XRender and GL; CPU, RSS, and Present pacing | Recorded 2026-10-05 at `ed2cdb0`, and the corners at `1fc0a1c`, with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-xorg-sessions-on-the-radeon-vega-desktop-2026-10-05). Physical hotplug and other window managers on Xorg are not recorded there. |
| Xorg with NVIDIA drivers or other AMD configurations | Pending | Requires a recorded server, window manager, driver, configuration, and commit; one AMD session does not establish other drivers. |
| XLibre with Intel/NVIDIA drivers or other AMD configurations | Pending | Requires the same environment evidence; one AMD session does not establish other drivers. |
| Xorg Xephyr 21.1.24 + Xmonad 0.18.1, nested in Xvfb, 1280×800×24 | Desktop scenarios with wallpaper change, idle/active CPU and RSS, Present, direct XRender, and effects modes, shutdown | Recorded 2026-10-03: [baseline](DESKTOP_TESTING.md#recorded-baseline-2026-10-03) with fade/blur disabled and an [effects baseline](DESKTOP_TESTING.md#recorded-effects-baseline-2026-10-03). No physical display or driver qualification. |
| XLibre Xephyr 25.1.9 + Xmonad 0.18.1, same virtual layout | Same desktop scenarios and measurements in all three modes | Recorded 2026-10-03; same configuration and limitations. Nested server results do not qualify an XLibre hardware session. |
| XLibre 25.1.9 native, modesetting + amdgpu, AMD Ryzen 5 5600GT (Radeon Vega, Mesa 26.2.4), Xmonad 0.18.1, HDMI + DP-to-VGA at 1920×1080 60 Hz | Mode, layout, and output changes; physical unplugging and reconnection of both connectors; Present and direct XRender; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with fade/blur disabled while the desktop's own applications ran. [Results and limitations](DESKTOP_TESTING.md#recorded-hardware-session-2026-10-03). The desktop scenario probe was not run in this session. |
| Dedicated XLibre 25.1.9 session on the same AMD machine, HDMI-1 alone at 1920×1080 60 Hz, glamor, default TearFree | Probe desktop scenarios with wallpaper change; Present, direct XRender, and effects (fades, translucency, blur); CPU, RSS, and Present pacing | Recorded 2026-10-03 with synthetic windows. [Results and limitations](DESKTOP_TESTING.md#recorded-hardware-desktop-session-2026-10-03). Convolution blur fell to about five frames per second behind a full-screen translucent window; the [pyramid blur](DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03) keeps 60. |
| XLibre 25.1.9 native, modesetting, AMD Ryzen 5 5600X + Radeon RX 9060 XT (Navi 44, radeonsi, Mesa 26.2.4), dwm 6.8, DP-2 + HDMI-1 at 1920×1080 60 Hz | Output on/off and layout changes with configuration reloads of blur, fades, and vsync; Present and direct XRender; CPU, RSS, XRes, and Present pacing | Recorded 2026-10-03 with synthetic windows while the desktop's own applications ran. [Results and limitations](DESKTOP_TESTING.md#recorded-reload-session-2026-10-03). No physical unplugging; the desktop scenario probe was not run. |
| Same RX 9060 XT desktop, one and two monitors | Compust benchmark scenes, then with region repaint, blur reuse, occlusion, and the GPU renderer beside XRender; CPU, GPU load, RSS, Present pacing, and open and close latencies | Recorded 2026-10-03 and 2026-10-04 with synthetic windows: [Compust benchmarks](DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03), [region repaint](DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04), [blur reuse](DESKTOP_TESTING.md#recorded-blur-reuse-2026-10-04), [occlusion](DESKTOP_TESTING.md#recorded-occlusion-2026-10-04), and [GPU renderer](DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04). Recorded before the release commit; the next row has the desktop scenarios. |
| Dedicated XLibre 25.1.9 sessions on the same RX 9060 XT desktop, DP-2 alone at 1920×1080 60 Hz, glamor, TearFree, with Xmonad 0.18.1, Openbox 3.6.1, i3 4.25.1, and bspwm 0.9.12 | Probe desktop scenarios with wallpaper change; Present, direct XRender, and effects (fades, translucency, blur); CPU, RSS, and Present pacing | Recorded 2026-10-04 with synthetic windows and the compositor source of 0.3.0-beta.1. [Results and limitations](DESKTOP_TESTING.md#recorded-desktop-sessions-on-the-rx-9060-xt-2026-10-04). A first bspwm session lost its last result when the machine froze; the second passed. |
| XLibre 25.1.9 on the RX 9060 XT, Xmonad 0.18.1, HDMI-1 at 60 Hz and DP-2 at 180 Hz | Pixels at monitor corners and seam, managed-window redraw, primary-output changes; XRender Present, direct XRender, and GL Present; CPU, RSS, XRes, and Present timing | Recorded 2026-10-04 at `a3e6f31`: [all nine samples passed](DESKTOP_TESTING.md#recorded-mixed-refresh-monitors-2026-10-04). Live applications kept drawing; the probe requests 60 updates/s with a 120-fps cap, so this does not qualify independent 180-fps output scheduling or long-term GL memory stability. |

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

The probe's scenarios pass on one AMD/XLibre session and one Intel/Xorg session with Xmonad, on the Intel/Xorg laptop with Openbox and i3, and on one AMD/Xorg session with i3. Extend hardware testing to further window managers, actual applications, and NVIDIA drivers. Record server, driver, configuration, and commit with every report. Cover reparenting after startup, rapid map/unmap/destroy sequences, decorated and override-redirect windows, menus, fullscreen transitions, wallpaper tools, and session shutdown.

Repeat physical hotplug and multiple-monitor layouts with other drivers and servers and more than two monitors, and measure longer-running memory and presentation behavior. [Mixed 60/180 Hz](DESKTOP_TESTING.md#recorded-mixed-refresh-monitors-2026-10-04) now has one short RX 9060 XT record; independent per-output pacing and other environments remain unqualified. The AMD/XLibre and Intel/Xorg monitor sessions, virtual RandR transitions, recovery from rejected or unfinished Present submissions, and repeated XRes accounting above are complete. Preserve the capture-request destruction, resource cleanup, large/off-screen shape, and malformed-property coverage recorded above.

**Acceptance:** documented reproductions become regression tests when feasible; ordinary desktop activity does not crash or leave invisible/stale windows; repeated lifecycle changes do not grow server resources without bound. Maintain a compatibility matrix with evidence instead of a blanket “supported” label.

## Rendering work: four steps done on one desktop

The four steps of this milestone are done on the RX 9060 XT desktop. Step 1 still lacks a second machine, which [1.0's performance step](#3-independent-performance-measurements) records. The beta showed where the cost was: Compust repainted the whole screen for every damage event, asked the server for the full window tree on every stacking-related event, and repeated the blur for every translucent window. Steps 2 and 3 removed the first two, and step 4 keeps each blur until something beneath it changes and skips what opaque windows hide. On the recorded machines a 60-updates-per-second window costs Compust under 2% of a core and the X server 3–9%, and the pyramid blur adds between a tenth of a point and one point to the server. The [first benchmark records](DESKTOP_TESTING.md#recorded-benchmark-scenes-2026-10-03) measure Compust's workloads on one machine; no record covers a 4K screen, many windows, or a slow GPU.

The milestone has four steps, in order. Steps 3 and 4 start only if step 1 shows that they matter.

| Step | Status | Required result |
| --- | --- | --- |
| 1. Independent benchmark scenes | Recorded on the RX 9060 XT desktop; further hardware pending | Fixed Compust scenes with workload, configuration, build identity, CPU, memory, GPU load, and frame-pacing records on two machines. |
| 2. Event-path round trips | Done; a regression counts the requests | Window events no longer cost one tree query each; a test counts the requests. |
| 3. Region-based repaint | Done; region frames match full repaints pixel for pixel | Only damaged regions, expanded for blur, are repainted and presented; pixel tests cover region boundaries. |
| 4. Occlusion and blur reuse | Done; each skip has a test of the change that undoes it | Fully covered windows are skipped and unchanged blur is reused, where the benchmark justifies it. |

### 1. Independent benchmark scenes

The [desktop probe](../examples/desktop_probe.rs) measures fixed Compust workloads: idle; a small window updating 60 times per second; a fullscreen translucent window, with and without blur; eight overlapping translucent windows; an opaque cover above them; moving and resizing; and repeated opening and closing. Record Compust and X-server CPU, GPU load where available, RSS, XRes, Present intervals, and open/close latency. Identify resolution, refresh rates, configuration, renderer, workload duration, and exact build.

**Acceptance:** two recorded machines have raw records of every Compust scene, the runner reproduces them, and the summary reports the measured costs and sampling limits. Repeat the same workload and configuration when measuring variation or a change to Compust.

**Status:** `--bench` and [`tools/bench.sh`](../tools/bench.sh) run Compust only, with XRender or its optional GL painter and with or without blur. Records on the RX 9060 XT cover the initial scenes and the later region repaint, blur reuse, occlusion, and GPU work. Runs and release criteria use Compust's independent measurements. The Intel/Xorg laptop is no longer available, so updated performance records on another machine remain open.

Moving and resizing raised Compust's own CPU from 0.3% to 1.1% in the initial record. Blur over eight windows added 0.6 points to Compust and 0.9 to the server; with step 4 it adds about 0.1 and 0.35. These own-workload measurements guide the following rendering work.

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

**Status:** done. With region repaint alone, blur over eight overlapping windows added 0.6–0.7 points to Compust and 1.2–1.9 to the X server on the RX 9060 XT desktop, [the largest remaining cost](DESKTOP_TESTING.md#recorded-region-repaint-2026-10-04) of that scene, because a change in the top window blurred every window beneath it again; a translucent terminal under blur did the same on each keystroke. Now every change records the lowest layer of the stack it alters, and each blurred window keeps its blurred backdrop. It blurs again only when a change beneath it reaches its footprint or the kept backdrop no longer matches its bounds; any other repaint copies the kept backdrop.

The [backdrop tests](../tests/cases/backdrops.rs) make each kind of change inside, above, beside, and beneath a blurred window, including restacks, an unmap, and moves of the window and of one beneath it, and compare every frame with one from a fresh renderer, with Present and with XRender. Changes inside or above the window present only their own area, a blurred window fading in and out blurs once, and backdrop buffers do not leak as windows map, resize, and close. In temporary copies, X11 tests failed without the re-blur for changes beneath, without the scene changes of restacks, departures, or former neighbors, with no kept backdrops, with a re-blur for a window's own changes, and with backdrops never dropped. One rule, that a window blurred again changes the scene of the windows above it, failed only its unit test: in the scenes tried, the footprint margin kept its effect from reaching the pixels above.

On the RX 9060 XT desktop, the eight-window scene with blur [cost Compust 0.50% instead of 1.02%](DESKTOP_TESTING.md#recorded-blur-reuse-2026-10-04) and the X server 0.8–1.2 points less; blur now adds about 0.1 points to Compust and 0.35 to the server over the same scene without blur. A full-screen translucent window with blur cost both 0.13–0.18 points less. The kept backdrops are server pixmaps, one per blurred window: Compust's pixmaps grew by 15 MB in the eight-window scene and by 25 MB in the full-screen one. All 80 X11 tests pass with Xorg's Xvfb 21.1.24, along with 23 unit and 6 CLI tests.

Windows hidden behind opaque ones are skipped. A window without an alpha channel at full opacity hides its shape from the windows and the background beneath it, except within the footprint of a blur due again above them, and a blurred window hidden whole drops its backdrop instead of blurring. The [occlusion tests](../tests/cases/occlusion.rs) count one RENDER Composite in a frame of an opaque full-screen window over three others. They compare frames from a fresh renderer with those after a cover moves aside and back, turns translucent and opaque, takes a hole, and goes away; compare a blur beneath a cover with the same scene without the cover; and uncover a blurred window hidden while the scene beneath it changed. In temporary copies, removing the hiding, the exception for blurs, or the dropped backdrop, letting translucent windows hide, or ignoring shapes each made an X11 test fail. In a new [benchmark scene](DESKTOP_TESTING.md#recorded-occlusion-2026-10-04) on the RX 9060 XT desktop, eight translucent windows under an opaque full-screen window that changes every frame cost Compust 0.23–0.25% instead of 0.43–0.50% and the X server 0.6–1.05 points less, with or without blur. All 84 X11 tests pass with Xorg's Xvfb 21.1.24, along with 28 unit and 6 CLI tests.

Step 3 kept the single Present buffer: outside the update region its contents already match the screen. Multiple presentation buffers with explicit ownership remain an open evaluation for latency. Input-to-display latency needs measuring equipment this project does not have; do not report it from software timings.

## 1.0: stable release

**Goal:** a release that a picom user on common Xorg or XLibre hardware can switch to for daily use, whose configuration stays valid across 1.x, and whose support claims rest on records. Stable does not mean that every driver works: every environment the release names has evidence, and the rest are listed as untested. This is the current milestone. It follows the rendering milestone above, and its step 3 also gives that milestone the second machine it lacks. It has no date.

### What 1.0 promises

- **Versions:** from 1.0.0, versions follow [semantic versioning](https://semver.org/). Every configuration field and command-line option that 1.0 accepts keeps working with the same meaning in every 1.x release. New ones may appear; removing or changing one waits for 2.0, after a 1.x release that warns about it.
- **Robustness:** ordinary desktop activity does not stop the compositor or leave a stale or invisible window. That covers windows that vanish between requests, malformed properties, monitor changes, suspend and resume, and a refused Present, each with a regression test or a recorded session.
- **Renderers:** XRender stays the default. The GPU renderer ships opt-in and falls back to XRender; it becomes the default only once recorded sessions on AMD, Intel, and NVIDIA show it drawing the same frames for no more CPU.
- **Claims:** the release names each qualified environment with its records and reports Compust's own measured costs and limitations for each workload.

### Steps

| Step | Status | Required result |
| --- | --- | --- |
| 1. Features picom users rely on | Released in 0.3.0-beta.2 and tested on Xvfb: per-window rules, [shadows](#10-step-1-shadows), [fullscreen unredirection](#10-step-1-fullscreen-unredirection), and [rules by focus](#10-step-1-rules-by-focus); fixed fixtures recorded in XRender and GL [on AMD/XLibre](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04) and [on AMD/Xorg](DESKTOP_TESTING.md#recorded-rounded-corners-on-the-radeon-vega-desktop-2026-10-05) | Shadows, fullscreen unredirection, and rules that choose by focus, with pixel tests in both painters |
| 2. Hardware and desktops | One AMD/XLibre desktop recorded with 0.3.0-beta.1 under five window managers; Radeon Vega/AMD and integrated Intel/Xorg already recorded with physical hotplug on both, and the Radeon Vega desktop [on Xorg under i3](DESKTOP_TESTING.md#recorded-xorg-sessions-on-the-radeon-vega-desktop-2026-10-05) since; updated candidate runs and NVIDIA pending | A release candidate recorded on AMD, Intel, and NVIDIA, on Xorg and XLibre, under six window managers with real applications |
| 3. Independent performance measurements | Compust scenes recorded on the RX 9060 XT before and after rendering changes; further hardware pending | Reproducible Compust CPU, GPU load, memory, resource, and frame-pacing records for each supported environment, without sustained backlogs or unexplained resource growth |
| 4. Endurance | Not started | 10,000 automated window cycles, and a week of daily use on two machines, without a crash or growing resources |
| 5. Outside testing | No outside reports yet | Three testers other than the maintainer run a release candidate, which then goes two weeks without a new crash, stale-window, or leak report |
| 6. Distribution and documentation | Release archives with checksums; [Open Build Service packages](../packaging/obs/README.md) for Debian, Ubuntu, Linux Mint, Fedora, and openSUSE | An Arch User Repository package, manual pages, a picom migration guide, troubleshooting, and a reviewed configuration |

### 1. Features picom users rely on

- **Shadows, [done](#10-step-1-shadows):** a soft shadow beneath windows, with global settings for its radius, offset, and opacity and a `shadow` setting for rules. Docks and desktop windows get none by default, and neither does a window that draws its own, such as a browser menu or a client-side-decorated GTK window. A shadow widens the area its window changes, never hides what lies beneath it, and fades with its window.
- **Fullscreen unredirection, [done](#10-step-1-fullscreen-unredirection):** an opt-in setting. While the topmost window is opaque and covers the whole root, Compust stops compositing: windows draw to the screen directly and the overlay is hidden. Compositing resumes as soon as that changes. One overlay covers every monitor, so a window that fills one of several monitors stays composited, and the documentation says so.
- **Rules by focus, [done](#10-step-1-rules-by-focus):** a `focused` selector, from the root's `_NET_ACTIVE_WINDOW`, so that rules can make inactive windows translucent, as picom's `inactive-opacity` does. The documentation lists window managers that do not set the property.

**Acceptance:** pixel tests cover each feature, in both painters where it draws, comparing frames with a fresh renderer's: shadow extent, offset, and shape; shadows of moving, fading, covered, and excluded windows; unredirection entered and left by mapping, unmapping, and resizing the fullscreen window and by stacking a translucent one above it, with correct pixels afterward; and focus moving between windows. The region repaint, blur reuse, and occlusion tests keep passing. Each new setting is documented in both languages.

### 2. Hardware and desktops

With a release candidate, record the probe scenarios, monitor changes, and the activities below on:

- AMD on Xorg and on XLibre, Intel on Xorg, and NVIDIA's proprietary driver on Xorg;
- one and two monitors, with physical hotplug on at least two of those machines and two monitors at different refresh rates on one;
- six window managers: Xmonad, Openbox, i3, bspwm, and dwm, recorded so far with betas, plus Xfwm4 with its own compositor off;
- real applications: a Firefox and a Chromium-based browser with their menus, a terminal, mpv windowed and fullscreen, a fullscreen game or OpenGL demo, a client-side-decorated GTK application, and an Electron application;
- workspace switches, entering and leaving fullscreen, drag and drop, a screen locker, suspend and resume, and logging out.

**Acceptance:** each environment has a record with its server, driver, window manager, configuration, and the release candidate's commit, and the compatibility matrix names only recorded environments. Each failure found is fixed, with a regression test where one can tell it apart, or listed as a known limit before the release. Records of earlier builds stay, but do not qualify 1.0.

### 3. Independent performance measurements

Run the [Compust benchmark scenes](DESKTOP_TESTING.md#run-the-benchmark-scenes) with a release candidate in each environment proposed for support. Record workload, resolution and refresh rates, painter, effects, exact build, duration, Compust and X-server CPU, GPU load where available, RSS, XRes, and Present intervals. Repeat equivalent runs to distinguish variation from regressions.

**Acceptance:** each declared environment has reproducible records for Compust's own workloads. Within the documented frame cap and presentation path, active scenes keep up without a sustained submission backlog or repeated recovery, and an idle controlled scene presents no frames. Repeated equivalent runs have no unexplained growth in idle RSS or owned pixmap bytes. Release notes state measured costs, sampling resolution, and known limits; no other compositor is a release gate.

### 4. Endurance

**Acceptance:** an automated run of 10,000 cycles that open, resize, and close windows, with reloads among them, ends with the XRes counts and owned pixmap bytes it had after warmup. Two machines run a release candidate as their daily compositor for seven days, sampling Compust's memory and the X server's resources every ten minutes, with no crash, no repeated Present timeouts, and idle memory on the last day within 5% of the first day's after warmup.

### 5. Outside testing

Publish betas as features land; [0.3.0-beta.1](#fourth-beta-prerelease) started with everything since 0.2.0-beta.3. Announce each release candidate where X11 users gather.

**Acceptance:** at least three testers other than the maintainer report running a release candidate through the issue templates, and that candidate then goes two weeks without a new report of a crash, a stale or invisible window, or a leak. Every report is fixed or listed as a known limit with its environment.

### 6. Distribution and documentation

**Acceptance:** an Arch User Repository package builds the release tag; release archives keep their checksums and reproducible packaging; manual pages cover the command and its configuration; a migration guide maps picom's common options to Compust's and names those without an equivalent; and a troubleshooting guide covers a refused start, the Present fallback, tearing, the GPU renderer's fallback, and logs. Before the first release candidate, every configuration field and command-line option is reviewed once, since 1.0 freezes them.

### Release path

1. **0.3.0-beta.1, [published](#fourth-beta-prerelease):** everything since 0.2.0-beta.3, namely configuration discovery and reload, region repaint, blur reuse, occlusion, the GPU renderer, weighted blur, and per-window rules, so that outside testing can start.
2. **0.3.0-beta.2, [published](#fifth-beta-prerelease):** step 1's three features, rounded corners, and reload on save, so that outside testing covers what 1.0 promises.
3. **Further 0.3 betas** as fixes and records land.
4. **1.0.0-rc.1:** step 1 is done and the configuration is reviewed and frozen; steps 2 to 5 run on release candidates.
5. **1.0.0:** a release candidate for which every step's acceptance holds, published unchanged.

### Not in 1.0

The [Window Animations](#window-animations-planned) milestone, multiple Present buffers, explicit synchronization, color management, HDR, VRR, and more than one X screen per process can come in 1.x releases that keep the 1.0 promises. Picom configuration compatibility and Wayland are outside the project's scope.

## Rendering backend and protocol expansion

Evaluate an EGL/OpenGL or Vulkan backend once the import and synchronization path is specified. Modern X11 integration may involve DRI3 DMA-BUF import, modifier negotiation, Present scheduling, and explicit synchronization. Each requires real implementation and driver testing; version probes do not count.

Color management, HDR, VRR, XLibre-specific extensions, and per-output scheduling require protocol and hardware investigations before support can be promised. Native Wayland is outside this roadmap.

**Acceptance:** a backend can import real application content without routine CPU readback, handles buffer lifetime correctly, recovers from supported failure modes, and has documented driver coverage and performance results.

**Status:** an OpenGL ES backend is implemented, opt-in with `backend = "gl"`, and [recorded on one desktop](DESKTOP_TESTING.md#recorded-gpu-renderer-2026-10-04). DRI3 Open selects the server's GPU, and DRI3 1.2 BuffersFromPixmap shares the back buffer, window, and wallpaper pixmaps as dma-bufs that EGL imports without copying, with the server's own modifiers, so nothing is negotiated. Implicit synchronization orders reads and writes once each side has sent its work, which a SYNC fence before each frame forces on the server's side, and Present shows the back buffer as before. Textures live as long as the captures whose pixmaps they show, and a renderer and its replacement keep separate contexts, which a test checks on Mesa's software device; CI's Xvfb servers lack DRI3, so the GPU screen painter is checked by the [dedicated hardware probe](DESKTOP_TESTING.md#recorded-shadows-focus-and-fullscreen-suspension-2026-10-04) instead. Missing DRI3, a renderer that cannot open, and a failed frame fall back to XRender for the session. On radeonsi with XLibre, the GPU renderer draws the same frames as XRender within two levels of color, at about the same total CPU, and costs more when windows resize; Xvfb lacks DRI3 and Xwayland refuses to share pixmaps, so both fall back. Intel and NVIDIA drivers, explicit synchronization through DRI3 1.4, and a recorded scene where the GPU renderer saves work remain open; until then XRender stays the default.

## Everyday usability

Beta use showed that changing `fade_ms` required restarting the compositor and that a configuration was read only when `--config` named it; [configuration discovery and reload](#everyday-usability-configuration-discovery-and-reload) now address both. [1.0](#10-stable-release) requires clearer troubleshooting and distribution packaging. [Per-window rules](#everyday-usability-per-window-rules-and-weighted-blur) now set opacity, blur, and fade duration by class, type, or title; the Window Animations milestone below expands the existing movement/scale proposal and would let rules choose animations too. Reuse that rule model for later effects. Shadows are part of 1.0, and the [Rounded Corners](#rounded-corners-implemented) milestone below arrived with them in 0.3.0-beta.2.

**Acceptance:** behavior is configurable, documented in both languages, testable, and does not silently claim compatibility with picom's configuration or scripting language.

## Window Animations: planned

**Goal:** generalize the implemented fade into a per-window animation system for opening and closing: fade (opacity), pop (scale plus opacity), and slide (translation), with configurable easing, chosen per window by the existing rules. This expands the animation work above. It is a proposed follow-up to the first beta, with no assigned release or date; the four beta gates remain unchanged.

### Starting point

| Area | Current implementation and consequence for this milestone |
| --- | --- |
| Fade | [animation.rs](../src/animation.rs) stores `from`, `to`, monotonic `started`, and `duration` in `Fade`. Integer smoothstep samples a `u16` opacity; close/reopen begin at the sampled value. Fade is implemented, including zero-duration endpoints and interrupted animations. Preserve this behavior. |
| Capture and close | [surface.rs](../src/surface.rs) uses `CompositeNameWindowPixmap` and an XRender `Picture`, not a GPU texture import. `Surface::close` marks the surface unmapped and retains its picture, named pixmap, geometry, shape, and Damage object. [scene.rs](../src/scene.rs) removes it after the fade finishes; `Surface`/`Picture` destructors release Damage, picture, and owned pixmap. A named pixmap already keeps the close contents alive after the original window disappears. |
| Remap and configure | `Scene::add` captures fresh mapped contents before replacing a closing surface, preserving its sampled fade and stacking. [events.rs](../src/events.rs) maps root children, closes on unmap/destroy, and handles reparenting. Configure updates position immediately; size/border changes recapture the pixmap while preserving the fade. Generalize those transfers to the complete animation state. |
| Render path | [paint.rs](../src/renderer/paint.rs) multiplies fade, window, and configured opacity and composites at the real window geometry, through XRender with an A8 mask or, with `backend = "gl"`, through the OpenGL ES shaders of [gpu.rs](../src/renderer/gpu.rs). Shape and [blur](../src/renderer/blur.rs) also use that geometry. Neither painter has a model-matrix path, shadow, or rounded-corner implementation. |
| Scheduling and damage | [compositor.rs](../src/compositor.rs) keeps painting while any fade is active, requests a final repaint, and then polls without continuous repainting. `max_fps` limits work; Present requires both completion and idle before reusing its single buffer. Each frame repaints only the area that changed, found by comparing each surface's shown state with the previous frame's; there is no buffer-age tracking. |
| Configuration and metadata | [config.rs](../src/config.rs) accepts strict TOML, with `fade_ms = 180` for both directions, and [rules.rs](../src/rules.rs) resolves ordered `[[rules]]` that match the cached client class, type, and title and set opacity, blur, and `fade_ms`. Unknown fields are rejected; SIGUSR1 reloads the file and resolves every window's rules again. Rules cannot choose an animation yet. Root property events are selected, but `_NET_CURRENT_DESKTOP` is not handled. |
| Outputs and exclusions | RandR currently triggers root-buffer recreation; there is no per-output geometry cache. Override-redirect tooltips are not excluded from fade by type. Fullscreen unredirection is not implemented, so fullscreen state must not be confused with an actually unredirected surface. |

### Scope and preparation

1. Replace `Fade` with an `Anim` state containing kind, start time, duration, easing, open/close direction, and sampled start/target transforms. Sample once per frame into `Transform { opacity, scale, offset }`. Retarget every component from its current value, including remap during close; do not restart from an endpoint or assume value continuity also preserves velocity.
2. Apply a center-based model transform only when rendering. For local point `p`, window origin `o`, and center `c` including the border, use `p_out = o + c + scale * (p - c) + offset`. Keep actual X window geometry and input regions unchanged. Prototype this on XRender with `SetPictureTransform` and transformed destination bounds/clips: its sampling matrix maps destination coordinates back to the source, so derive the inverse and account for Composite source/destination origins. Verify filtering, identity reset, and checked fixed-point conversion before adding effects. A GPU backend is not a prerequisite. See the [Render protocol](https://xorg.freedesktop.org/archive/current/doc/renderproto/renderproto.txt).
3. Pop opens from approximately `scale = 0.85`, opacity zero, to identity and full animation opacity; closing targets the small transparent state. Offer `ease_out_cubic` or `ease_out_back` for opening and `ease_in_cubic` for closing. Keep legacy `smoothstep`; add `linear` and the cubic in/out and back-out curves as pure calculations. Clamp opacity, keep scale positive/invertible, and include back-curve overshoot in painted bounds. A spring model is optional future work.
4. Slide opens from the nearest edge of the window's current output and closes toward the selected edge; allow top/right/bottom/left overrides. Select the active RandR output with the greatest window intersection, use a deterministic tie-break, and freeze the selection for a closing snapshot. Handle negative origins, overlapping outputs, spanning windows, and output changes explicitly. If output geometry cannot be established, use fade rather than treating a multi-monitor root as one monitor. Dropdown menus can use a short displacement, proposed at 24 pixels, instead of a full edge traversal.
5. Before implementation, confirm retained-resource ownership on unmap/destroy, failed recapture, remap, and shutdown against the existing [fade](../tests/cases/fades.rs), [capture-race](../tests/cases/capture_races.rs), and [resource](../tests/cases/resources.rs) coverage. Preserve the last successful capture and its metadata until completion; keep destroyed snapshots in their established stacking position. Reuse owned pictures and masks instead of naming/copying pixmaps on every animation frame.
6. Region repaint compares each surface's shown state between frames. Make the sampled transform part of that state and the surface's bounds its transformed bounds, including back-curve overshoot, so every animation frame repaints the union of the previous and current transformed bounds, widened for blur as today and for any future shadow. Preserve Present buffer ownership, direct-XRender fallback, frame limiting, and return to the existing idle poll, including the final cleanup frame. Confirm these paths for several simultaneous animations and zero duration.

### Proposed configuration and eligibility

Extend the existing TOML format with `[animations]` and ordered `[[animation_rules]]`, or add the animation fields to the existing `[[rules]]`. **These fields are a proposal and are not accepted by the current binary.** Keep `compust.example.toml` valid until implementation. The owner must confirm the public schema before coding, following the [feature proposal template](../.github/ISSUE_TEMPLATE/feature.yml).

- Global fields: `kind` (`none`, `fade`, `pop`, `slide`), separate `open_ms`/`close_ms`, `open_easing`/`close_easing`, `pop_scale` (default `0.85`), `slide_direction` (`nearest` by default, or `top`, `right`, `bottom`, `left`), optional `slide_offset_px` (absent means edge traversal), and `suppress_workspace_switch` (proposed default `true`).
- Compatibility defaults: fade, smoothstep in both directions, and durations inherited from `fade_ms` (180 ms when absent). Explicit new duration fields override that legacy value per direction; zero completes immediately. Proposed opt-in pop/slide examples use 220 ms open and 150 ms close. The refactor preserves eligible-window fade output exactly; new exclusions and workspace suppression are intentional eligibility changes.
- Rules match as the existing `[[rules]]` do: the cached client `WM_CLASS` resource class, `_NET_WM_WINDOW_TYPE`, and name (`_NET_WM_NAME`, falling back to `WM_NAME`), through the client/frame association, with validated properties and metadata kept after destruction. Matching is case-sensitive exact text, all supplied selectors must match, and each field comes from the first matching rule that sets it. Missing/malformed properties do not satisfy a selector, except that a window without a type takes its EWMH default type. Refresh metadata for future transitions without restarting an active animation merely because a title changes.
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
3. **Add animation configuration and exclusions.** **Problem or use case:** global `fade_ms` and the existing rules cannot choose an animation. **Proposed behavior:** extend `config.rs` and the rules of `rules.rs`, which already match client metadata with first-match precedence, with strict TOML for animation kinds, separate open/close settings, and exclusions; retain legacy defaults. **How to verify it:** parsing/precedence cases cover valid, absent, malformed, and conflicting data; frame/client and destroyed-client cases select the intended policy. Expose each new animation kind only when its rendering task is ready.
4. **Introduce the XRender transform path.** **Problem or use case:** painting/blur/clips use untransformed geometry. **Proposed behavior:** add center-based affine sampling, inverse picture transforms, destination bounds, consistent effect coverage, and reusable transform/filter state in `picture.rs` and `renderer/{paint,blur}.rs`. **How to verify it:** identity matches existing pixels; controlled scale/offset samples preserve centers, shapes, borders, transparency, blur, and extreme off-screen clipping without per-frame resource growth.
5. **Implement pop and selectable easing.** **Problem or use case:** the generic path needs a complete scale/opacity effect. **Proposed behavior:** add the 0.85-to-1 pop endpoints, cubic/back/linear sampling, bounded opacity and scale, and configurable open/close curves. **How to verify it:** pure endpoint/overshoot/interruption cases and real-server open, unmap, destroy, and remap scenes pass with preserved last contents.
6. **Implement output-aware slide and menu offsets.** **Problem or use case:** root-wide edges are wrong on multiple monitors. **Proposed behavior:** cache negotiated RandR monitor/CRTC geometry, refresh on output/CRTC changes, select the window's output, and add nearest/explicit directions, dropdown offsets, and documented fallback. **How to verify it:** deterministic geometry cases plus pixel captures cover different resolutions, negative origins, ties, spanning windows, hotplug/resize during animation, and missing RandR information.
7. **Suppress workspace-driven animations.** **Problem or use case:** a workspace transition can resemble many independent window openings/closings. **Proposed behavior:** add root desktop tracking and bounded lifecycle classification in `atoms.rs`, `events.rs`, and `compositor.rs`; settle affected snapshots and preserve genuine later transitions. **How to verify it:** reordered/batched protocol scenarios and an actual Xmonad session pass switch, rapid-switch, interrupted-animation, and subsequent ordinary-open checks; record unsupported WM behavior.
8. **Qualify performance, cleanup, and documentation.** **Problem or use case:** visible correctness alone cannot establish idle behavior or resource stability. **Proposed behavior:** extend `tests/cases/`, the XRes helpers, and the desktop probe for concurrent transforms, repeated lifecycle changes, and measured pacing; update the README, architecture, valid example TOML, and both roadmaps when implemented. **How to verify it:** every acceptance checkbox has evidence for the exact commit and declared environment; proposed-only features remain labeled planned until their checks pass.

### Decisions and later work

Owner confirmation is still needed for the proposed `[animations]` schema, whether animation settings join `[[rules]]` or a separate `[[animation_rules]]` table, and the opt-in 220/150 ms presets; first-match exact-text matching was confirmed for `[[rules]]`. The plan preserves the 180 ms legacy fade default and proposes enabling workspace suppression. No new release date or beta gate is assigned.

Animating move/resize geometry of existing windows and whole-workspace slide transitions are out of scope and remain future roadmap items. A spring model, shadows, rounded corners, and fullscreen unredirection remain separate work.

## Rounded Corners: implemented

**Goal:** round the corners of windows, with a global `corner_radius` and the same field in per-window rules, so that a window, the blur behind it, its shadow, and what it hides follow one rounded outline in both painters. Rounded corners were planned after 1.0 and landed in [0.3.0-beta.2](#fifth-beta-prerelease) instead, so `corner_radius` is among the settings that 1.0 reviews and freezes. Task 6, which qualifies them, remains.

### Starting point

| Area | Current implementation and consequence for this milestone |
| --- | --- |
| Shape | [surface.rs](../src/surface.rs) keeps a window's bounding shape as rectangles, border included, and knows whether that shape is its whole rectangle. Clips and the repaint area are lists of rectangles. A rounded edge needs partial coverage at each pixel, which rectangles cannot express without jagged edges, so it belongs in a mask while clips stay rectangular. |
| XRender painting | [paint.rs](../src/renderer/paint.rs) composites each surface `OVER` the back buffer through a 1×1 repeating A8 picture that holds its opacity, and shows a blurred backdrop through weights of the window's alpha times that opacity. Neither mask holds a coverage that varies across the window. |
| GPU painting | [gpu.rs](../src/renderer/gpu.rs) draws through `compust-gl`'s `draw`, `draw_masked`, and `shade`, which sample a source, and a mask where there is one, at `(target pixel + offset) × scale`. The two painters draw the same frames within two levels of color, and rounded corners must keep that. |
| Shadows | [shadow.rs](../src/renderer/shadow.rs) blurs the window's rectangle with three integer box filters, which separates into one strip per axis; a rounded rectangle does not separate near its corners. The shadow is painted around the window's bounds and never beneath them, so the gaps a rounded corner leaves inside the bounds would show none. Shaped windows cast no shadow. |
| Occlusion | [cover.rs](../src/renderer/cover.rs) lets a surface without alpha, at full opacity, hide its shape from everything beneath it. A rounded window no longer covers its corners, where the scene beneath must still be painted. |
| Damage | [damage.rs](../src/renderer/damage.rs) compares each surface's shown bounds, shape, picture, opacity, and blur with the previous frame's. Corners lie inside the bounds, so a radius that a reload or a focus rule changes only needs adding to that state. |
| Fullscreen unredirection | [compositor.rs](../src/compositor.rs) suspends compositing only for a topmost surface that is opaque, wholly rectangular, and covers the screen. A rounded window is not wholly rectangular, so a fullscreen window must stay square or unredirection would never apply to it. Compust does not read `_NET_WM_STATE` today. |
| Rules | [rules.rs](../src/rules.rs) resolves ordered `[[rules]]` by class, type, title, and focus, and sets opacity, blur, fade duration, and shadow. A client's type and its `_GTK_FRAME_EXTENTS` margins decide whether it casts a shadow unless a rule says otherwise; the same identity can decide which windows are rounded. |

### Configuration

**The owner confirmed this schema in [task 1](#task-1-done-schema-and-corner-method). `main` draws rounded corners since [task 5](#task-5-done-corners-on-the-gpu); 0.3.0-beta.1 rejects these fields.**

- `corner_radius`: a global radius in pixels, 0–64. Zero, the default, rounds nothing, so an upgrade changes no desktop, as with shadows.
- A rule's `corner_radius`, in the same range, sets the radius of the windows it chooses whatever their type or margins; `corner_radius = 0` keeps them square. Each setting comes from the first matching rule that sets it, as for the existing fields, and a reload resolves it again.
- Without a rule, a window is rounded when it would cast a shadow: its type is `normal`, `dialog`, `utility`, `splash`, or `toolbar`, and it declares no `_GTK_FRAME_EXTENTS` margins, inside which a client-side-decorated window draws its own corners. Docks, desktops, menus, tooltips, and notifications stay square unless a rule rounds them.
- A window whose bounding shape is not its whole rectangle keeps that shape and is not rounded. A window whose client the window manager marks `_NET_WM_STATE_FULLSCREEN` stays square, so fullscreen unredirection still applies to it, on one monitor of several too. Compust reads `_NET_WM_STATE` with the rest of the client's identity, and again when it changes.
- When painting, the radius is limited to half the window's shorter side, border included, so a small window's arcs never overlap.

Example, which `main` accepts without drawing it yet:

```toml
corner_radius = 8

# Square terminals.
[[rules]]
wm_class = "Alacritty"
corner_radius = 0

# Rounded notifications, which their type would leave square.
[[rules]]
window_type = "notification"
corner_radius = 12
```

### Proposed rendering

1. **Coverage table.** For each radius in use, compute one corner's coverage as an `r × r` table: how many of 16 × 16 sample points in each pixel lie inside the quarter circle, counted with integers and scaled to 0–255, so both painters read the same values. The four corners are the quadrants of one `2r × 2r` disk. Keep one disk per radius in use, and release it once no surface uses that radius.
2. **XRender, decided in task 1.** Upload each disk once as an A8 picture. At full opacity it is the corner mask itself; otherwise one composite of the disk through the 1×1 opacity mask, which the frame fills already, writes coverage times opacity into a scratch A8 picture of the disk's size. Paint the window without its four corner squares through the opacity mask, under a clip that leaves them out, then each corner square through its quadrant of the corner mask. Multiply the backdrop's weights by the disk's quadrants with `IN`, so blur never shows in the cut corners; a backdrop without weights takes the corner mask as the window does.
3. **GPU.** Upload the table as a texture, and add a draw that multiplies it into the corner squares of both the window and its backdrop; the rest of the window keeps its current draws. A distance computed in the shader would be simpler, but would not match XRender's values.
4. **Occlusion.** A rounded opaque surface hides its shape without its four corner squares, which never hides more than it covers. The scene beneath the corners is painted as beneath a translucent window.
5. **Shadows.** Edges keep their strips. Each corner gets a square tile, `corner_radius + 2 × shadow_radius` pixels on a side, of the rounded corner blurred by the same three box filters in two dimensions, computed once per pair of radii. The shadow also fills the corner gaps inside the window's bounds, through the complement of the coverage, so no background shows between a window and its shadow and a translucent window is still no darker for its own shadow.
6. **Damage, done in task 2.** Add the radius to what a frame showed of each surface. A change repaints the surface's bounds and, with a shadow, its extent.

### Interaction and risks

- **Borders:** X window borders, which Xmonad draws, are part of the window's rectangle. Rounding cuts their outer edge while their inner edge stays square, so a thick border looks uneven at the corners. Rounding the inner edge too needs the border width in the mask and is later work; document the limit.
- **Tiling layouts:** windows that touch their neighbors show the wallpaper or the window beneath in each corner. A rule with `window_type = "normal"` and `corner_radius = 0` keeps tiled windows square.
- **Window Animations:** the [planned transforms](#window-animations-planned) must carry the corner mask with the window, as they carry its shape and blur. Whichever milestone lands second runs the other's pixel tests through its own change.
- **Translucent and ARGB windows:** coverage multiplies the window's own alpha, so a menu's transparent margin stays transparent while its rounded edge stays smooth.
- **Cost:** each frame that repaints a rounded surface composites four more corner squares for the window, its backdrop, and its shadow. Measure Compust's and the X server's CPU with corners on and off before claiming a cost.

### Verification and acceptance

- [ ] With `corner_radius = 0` and no rule setting it, both painters draw exactly the frames they draw today, and every existing test passes unchanged.
- [ ] Unit tests cover the coverage table: full inside the arc, empty outside it, symmetric, monotonic along each row and column, and a radius limited to half the shorter side, including 1×1 surfaces.
- [ ] X11 pixel tests in XRender check each corner of opaque, translucent, and ARGB windows, with and without a border and during fades, against an independent reference, with the interior and the edges away from the corners unchanged.
- [ ] Blur is cut at the corners. The scene beneath a rounded opaque window shows through its corners after moves, restacks, and reloads, matching a fresh renderer's frames.
- [ ] Shadows fill the corner gaps without darkening a translucent window, match a two-dimensional reference blur at the corners, and keep their current values along the edges.
- [ ] Rules round and square windows by class, type, title, and focus; a focus change or a reload that changes a radius repaints the right area. Client-side-decorated, shaped, and fullscreen windows stay as they are, and fullscreen unredirection still applies.
- [ ] The GPU painter draws the same frames as XRender within two levels of color, on Mesa's software device and in the hardware probe.
- [ ] After warmup and 1,000 open and close cycles with rounded corners, XRes counts and owned pixmap bytes return to their settled values; a reload that changes the radius releases the tables no surface uses.
- [ ] The benchmark scenes record Compust's and the X server's CPU with corners on and off, in both painters, on a recorded desktop.
- [ ] Formatting, strict Clippy, the full suite, the release build, and documentation in both languages, including the README's settings and rules tables and `compust.example.toml`, pass before the milestone is advertised as implemented.

### Ordered tasks

Each item is one reviewable issue, with the feature template's fields: **Problem or use case**, **Proposed behavior**, and **How to verify it**. The last field is the done criterion.

1. **Confirm the schema and the mask method, [done](#task-1-done-schema-and-corner-method).** **Problem or use case:** once 1.0 is out, a configuration field cannot change within 1.x. **Proposed behavior:** confirm the field's name, range, and default by type, the fullscreen and shaped-window policies, and the XRender corner method, after counting its requests per frame. **How to verify it:** each open question in this section has a recorded decision.
2. **Add the setting and the coverage table, [done](#task-2-done-setting-and-coverage-table).** **Problem or use case:** nothing chooses or computes a radius. **Proposed behavior:** add `corner_radius` to `config.rs` and `rules.rs`, read the client's fullscreen state with its identity, and compute the table in a new `renderer/corner.rs`. **How to verify it:** parsing, precedence, identity, and table tests pass, and frames with the default configuration do not change.
3. **Round corners in XRender, [done](#task-3-done-corners-in-xrender).** **Problem or use case:** the opacity mask and the backdrop weights are uniform across the corners. **Proposed behavior:** corner masks for the window and its backdrop in `paint.rs`, and corners left out of what `cover.rs` hides. **How to verify it:** the pixel, blur, occlusion, and region repaint tests above pass on Xvfb.
4. **Round shadows, [done](#task-4-done-rounded-shadows).** **Problem or use case:** shadow strips cannot draw a rounded corner, and nothing fills the gaps. **Proposed behavior:** corner tiles and gap fill in `shadow.rs`, in both painters. **How to verify it:** the shadow tests above pass, and existing shadow tests pass unchanged.
5. **Round corners in the GPU painter, [done](#task-5-done-corners-on-the-gpu).** **Problem or use case:** `compust-gl` has no draw that multiplies a coverage texture. **Proposed behavior:** the coverage texture and corner draws in `gpu.rs` and `crates/gl`. **How to verify it:** parity tests pass on Mesa's software device and in the hardware probe.
6. **Qualify and document.** **Problem or use case:** pixel tests alone do not show cost or resource stability. **Proposed behavior:** resource cycles in `tests/cases/`, benchmark runs on a recorded desktop, and updates to the README, the architecture, `compust.example.toml`, and both roadmaps. **How to verify it:** every acceptance item has evidence for the exact commit and environment.

### Task 1 done: schema and corner method

The owner confirmed on 2026-10-05: the field is `corner_radius`, 0–64, both global and in rules; without a rule, windows are rounded when they would cast a shadow; fullscreen windows, found by `_NET_WM_STATE_FULLSCREEN`, and shaped windows stay square.

A throwaway program, kept out of the repository, drew a 200×150 window with rounded corners onto a black 320×240×24 buffer on Xvfb 21.1.24 in the order step 2 describes, and compared every pixel of the buffer with values computed on the CPU with pixman's rounding. Every pixel matched for radii 1, 2, 8, 13, and 64, for an opaque window and a half-transparent ARGB one, and for opacities `0xff`, `0x99`, and `0x01`. Building the scratch mask by filling it with the opacity and multiplying the disk in with `IN`, as first proposed, drew the same pixels with one request more. Four `IN` composites of the disk's quadrants multiplied an A8 weights picture by the coverage exactly at every pixel. The tables were symmetric and never decreased toward the inside for radii 1, 2, 3, 8, 13, and 64; from radius 4 the outermost pixel is empty, and the innermost is full.

| Requests for one rounded window in a frame | Today | With rounded corners |
| --- | --- | --- |
| At full opacity | 3 | 8: one more clip and four corner composites |
| Translucent or fading | 3 | 9: the scratch mask as well |
| Showing a blurred backdrop beneath it | 1 more, or 3 with weights | 4 more than today, or 5 with weights |

The first two rows were counted by the program; the backdrop row is counted from the code and step 2's order, without the blur itself. None of these requests waits for a reply. The largest disk and its scratch picture, for radius 64, are 128×128 A8 pictures of 16 KiB each. Tasks 2 and 3 turn these checks into the repository's unit and X11 tests.

### Task 2 done: setting and coverage table

`corner_radius` is a global setting and a rule field, each 0–64 and rejected outside it; a rule's radius, zero included, counts as a setting. Each surface's radius comes from [`rules::corner_radius`](../src/rules.rs): its rule's, or else the global one when the window is decorated, the same condition that gives it a shadow, which the identity now calls `decorated`. A fullscreen window, whose client's `_NET_WM_STATE` lists `_NET_WM_STATE_FULLSCREEN`, gets none whatever its rules say, and neither does a shaped one. The identity reads `_NET_WM_STATE` as a seventh property in the same round trip, as a list of at most 32 atoms, and again when it changes. [corner.rs](../src/renderer/corner.rs) limits the radius to half the surface's shorter side, border included, and computes the coverage disk as task 1 measured it. What a frame showed of a surface includes its radius, so step 6 is done here rather than in task 3. Nothing paints with the radius or the disk yet; the disk carries an `expect(dead_code)` that task 3 has to remove, since the attribute fails the build once something uses the disk.

Five unit tests cover the disk's symmetry, growth toward the middle, exact values for radii 1 and 8, and area within a sixteenth of a pixel per pixel of radius; the limit; rule precedence with decoration and fullscreen; and `_NET_WM_STATE` replies of the wrong type, format, or length. An X11 test in [rules.rs](../tests/cases/rules.rs) sets valid, wrongly typed, 8-bit, overlong, and empty states, each followed by a class change the compositor must follow, then destroys the window right after a new state and maps another; temporary copies that panic on an 8-bit state or on any list of atoms make it fail. The README documents the setting once task 3 draws it. All 122 X11 tests pass with Xorg's Xvfb 21.1.24, along with 40 unit and 6 CLI tests, strict Clippy, and the release build.

### Task 3 done: corners in XRender

The XRender painter draws rounded corners as task 1 decided: one A8 disk per radius in use, uploaded once and released when no surface uses it, and one 128×128 scratch picture for corners below full opacity. A rounded surface's interior is drawn under a clip that leaves out its corner squares, and each square through its quadrant of the disk or the scratch picture. Its blurred backdrop shows through weights that four `IN` composites multiply by the disk, so blur never shows beyond the arcs. A rounded opaque surface hides only its interior from the surfaces beneath it, and a rounded surface never counts as covering the screen, so fullscreen unredirection waits for the square corners of a fullscreen window. Task 5 rounds the GPU painter's corners.

Seven X11 tests in [corners.rs](../tests/cases/corners.rs) check every pixel of opaque, half-transparent, ARGB, and bordered windows and the two pixels around them against coverage integrated independently across each pixel, within four levels where an arc crosses a pixel and within rounding elsewhere; blur beyond the arcs; a window beneath that shows and redraws through the corners of an opaque one, against a full repaint; rules that round and square windows, with menus, client-side-decorated, fullscreen, and shaped windows square and a reload that squares them all; a rounded window over the whole screen that stays composited until it becomes fullscreen; and disks released as radii change. In temporary copies, leaving the backdrop weights square, letting rounded windows hide their corners or cover the screen, swapping two quadrants, ignoring opacity at the corners, or never releasing disks each made a test fail. All 130 X11 tests pass with Xorg's Xvfb 21.1.24, along with 46 unit and 6 CLI tests and strict Clippy.

### Task 4 done: rounded shadows

A rounded surface's shadow is the blur of its rounded shape. The blur is linear, so it equals the blur of the rectangle, which the strips draw, less the blur of each corner that the rounding cuts off. That correction depends only on the corner and shadow radii: [shadow.rs](../src/renderer/shadow.rs) computes it once per pair, with the profiles' integer kernel in one pass along each axis, and mirrors it to the other corners. Near each corner a patch of up to `corners + 2 × shadow_radius` pixels holds the corrected values, together with the surface's corner square, which the shadow fills as far as the surface leaves each pixel uncovered; patches that would overlap on small surfaces merge, and the strips draw everywhere else. The patches are made again only when the surface's size, the radii, or the offset change, and the correction only when a radius does.

A unit test compares the strips' product with the patches drawn over it against a direct two-dimensional blur of the rounded surface, pixel by pixel, for six sizes, radii, corners, and offsets, within two levels, and checks that patches never overlap. Two X11 tests in [corners.rs](../tests/cases/corners.rs) compare every pixel around a rounded window with an offset shadow against the documented three-box blur of its exactly integrated shape, within four levels, and check that a translucent rounded window is no darker for its shadow and that a moved one matches a full repaint. In temporary copies, drawing no patches, leaving out the correction, filling the gaps without the window's coverage, or drawing the strips beneath the patches each made a test fail. All 132 X11 tests pass with Xorg's Xvfb 21.1.24, along with 47 unit and 6 CLI tests and strict Clippy.

### Task 5 done: corners on the GPU

The GPU painter draws rounded corners from one disk texture per radius, with two new draws in `compust-gl`: one composites a window through the disk, keeping the window's own alpha, and one shows a blurred backdrop through the window's alpha times the disk. A rounded shadow's patches go through the existing shadow draw, with an opaque one-pixel texture as the second profile. Two tests in the GPU crate check both draws on Mesa's software device: a half-transparent premultiplied source keeps its alpha through three mask levels, and the backdrop draw weighs by both alphas, with the disk placed independently of the mask.

Xvfb has no DRI3, so the X11 tests exercise only the XRender painter. The [feature probe](DESKTOP_TESTING.md#check-shadows-focus-and-fullscreen-suspension) gains a corners scene for that reason: an opaque and a half-transparent rounded window over a checkered pattern, with their shadows and blur, which the GL run compares with the XRender run within two levels per channel. Its configuration now blurs at radius 4 and rounds the scene's windows by class, which leaves the shadow scene's pixels as they were: an Xvfb rehearsal passed every scene, the shadow scene with no difference from its reference. On the Radeon Vega desktop with Xorg, the [first hardware run](DESKTOP_TESTING.md#recorded-rounded-corners-on-the-radeon-vega-desktop-2026-10-05) found that GL blended a translucent window twice where clip rectangles overlapped, an error older than the corners, which `1fc0a1c` fixes; the second run passed every scene, with GL within one level of XRender on the corners scene.

### Decisions and later work

Rounded borders now follow an inner arc inset by the X border width. The border color comes from the surface's top-left border pixel, so window managers that paint different colors along each edge cannot preserve those colors at the corners. Different radii per corner and rounding windows that already have a shape are later work.

## Active and Inactive Opacity: planned

**Goal:** make the opacity of active and inactive windows a first-class setting, as picom's `active-opacity`, `inactive-opacity`, and `inactive-dim` are, with transitions that ease instead of jumping. [Rules by focus](#10-step-1-rules-by-focus) already let a rule with `focused = false` make inactive windows translucent; this milestone adds global settings that leave menus, tooltips, and docks alone, darkening as an alternative to translucency, and eased changes. It is not required for 1.0. New configuration fields keep 1.0's promises, so it can land before or after 1.0; it has no assigned release or date.

### Starting point

| Area | Current implementation and consequence for this milestone |
| --- | --- |
| Focus | [rules.rs](../src/rules.rs) gives rules a `focused` selector from the root's `_NET_ACTIVE_WINDOW`. A change costs one read and resolves the rules again for the windows that lost and gained focus; a frame is focused with its client. Without the property every window counts as focused, so nothing dims: Xmonad's default configuration, without `XMonad.Hooks.EwmhDesktops`, is one such window manager. |
| Opacity | [paint.rs](../src/renderer/paint.rs) multiplies, on every frame, the application's `_NET_WM_WINDOW_OPACITY`, the rule's or else the global `opacity`, and the open or close fade. A focus change, a reload, or a new application opacity therefore shows at the next frame: the window jumps to its new opacity. |
| Animation | [animation.rs](../src/animation.rs) eases each fade with integer smoothstep and retargets it from the sampled value, so an interrupted fade stays continuous; [scene.rs](../src/scene.rs) keeps painting while any fade runs. A second value of the same kind, for the configured opacity, fits this model. |
| Eligibility | A rule with `focused = false` also matches menus, tooltips, and docks, so the README advises adding `window_type = "normal"`. Each identity already knows whether Compust decorates the window, the condition for shadows and rounded corners, and whether its client is fullscreen. |
| Effects | Blur shows behind a window below full opacity; only a window at full opacity without alpha hides what lies beneath it; a shadow's darkness multiplies its window's opacity; fullscreen unredirection needs the topmost window at full opacity. An inactive window made translucent therefore costs a blur and stops hiding the windows beneath it, while a darkened one stays opaque. |
| Painters | The XRender painter composites each window through a 1×1 A8 mask that holds its opacity, and the GPU painter multiplies the same value in its shader. Neither has a step that darkens a window. |

### Configuration

**The owner confirmed this schema in [task 1](#task-1-done-schema-and-semantics); the current binary does not accept these fields yet.**

- `active_opacity` and `inactive_opacity`: percentages, 0–100, that an eligible window takes in place of the global `opacity` while it is, or is not, the active window. Absent, the default, they leave the global `opacity` in place, so an upgrade changes no desktop. A rule's `opacity` still wins over both, as it wins over the global one, and the application's opacity still multiplies the result.
- `inactive_dim`: how much darker an eligible inactive window is drawn, as a percentage of black over it, 0–100, default 0. A rule's `dim`, in the same range, sets it for the windows the rule chooses, which a `focused` selector can narrow.
- Eligible windows are those Compust decorates, as for shadows and rounded corners: types `normal`, `dialog`, `utility`, `splash`, and `toolbar` without client-side margins. A window whose client is fullscreen is not eligible, so it stays opaque and fullscreen unredirection still applies to it. Menus, tooltips, docks, and the other types count as active for these settings; a rule can still choose any of them by focus.
- Every change of a window's opacity or dimming eases over its fade duration, its rule's `fade_ms` or else the global one, with the smoothstep of the open and close fades, starting from the value it shows at that moment. That covers a focus change, a reload, and a new `_NET_WM_WINDOW_OPACITY`. With `fade_ms = 0`, changes stay immediate, as today.

Example, which the current binary does not accept yet:

```toml
inactive_opacity = 85
inactive_dim = 10

# Terminals keep their look when inactive.
[[rules]]
wm_class = "Alacritty"
focused = false
opacity = 100
dim = 0
```

### Proposed behavior

1. **Eased opacity.** Give each surface a retargetable value for its configured opacity and dimming beside its open and close fade, sampled once per frame. A new target starts from the current sample, so focus moving back during a transition reverses it without a jump. The scene keeps painting while any of these values moves, and stops when all have settled.
2. **Active and inactive opacity.** Resolve each surface's configured opacity as its rule's `opacity`, else `active_opacity` or `inactive_opacity` when it is eligible, else the global `opacity`, and retarget it whenever focus, rules, the identity, or the configuration change.
3. **Dimming in XRender.** After compositing an eligible inactive window, composite black over its clip at the dim strength times its opacity, through the window's own alpha for an ARGB window, so that a transparent margin stays transparent. With rounded corners, the corner mask applies to the dimming as to the window.
4. **Dimming on the GPU.** Multiply the window's color by one minus the dim strength in the same draw, so the two painters agree within two levels of color, as they do today.
5. **Damage.** What a frame showed of a surface already includes its opacity; add its dimming, so each step of a transition repaints the surface's bounds and, with a shadow, its extent.
6. **Focus without `_NET_ACTIVE_WINDOW`, decided in task 1.** Under a window manager that does not set the property, follow X input focus instead: the client that holds it, or none when focus is on the root or follows the pointer. The property still wins wherever the window manager sets it.

### Interaction and risks

- **Cost:** with `inactive_opacity` below 100, every window but one is translucent: each blurs the scene beneath it when blur is on, and none hides the windows beneath it. Measure Compust's and the X server's CPU against `inactive_dim`, which keeps windows opaque, and recommend dimming where translucency costs too much.
- **Workspace switches:** focus moves on every switch, so the windows of the new workspace ease at once. The number of transitions is bounded by the windows shown; measure frames during switches with several windows.
- **Multiple monitors:** an inactive window on another monitor dims too. That matches picom and is documented, not configurable, in this milestone.
- **Shadows:** a shadow's darkness already follows its window's opacity, so an inactive translucent window casts a lighter shadow; dimming leaves the shadow as it is.
- **Window Animations and Rounded Corners:** dimming must follow the transforms of the [Window Animations](#window-animations-planned) milestone and the corner mask of [Rounded Corners](#rounded-corners-implemented); whichever lands second runs the other's pixel tests through its change. Eased opacity may share the generalized animation state that Window Animations proposes.
- **Input focus:** following X input focus, focus on the root, `PointerRoot`, a frame, or a window gone before its focus event is read must each count as no active client, never dimming the whole desktop by mistake.

### Verification and acceptance

- [ ] Without `active_opacity`, `inactive_opacity`, `inactive_dim`, or a rule's `dim`, both painters draw exactly the frames they draw today once transitions settle, and with `fade_ms = 0` they draw them at the same moments; every existing test passes unchanged.
- [ ] Unit tests cover eased opacity: exact endpoints, zero duration, a retarget halfway that reverses without a jump, and several targets in one frame.
- [ ] X11 pixel tests in both painters cover active and inactive opacity, precedence with rule and global opacity, application opacity multiplied in, and eligibility: menus, tooltips, docks, client-side-decorated, and fullscreen windows stay as they are.
- [ ] Dimmed pixels of opaque, ARGB, and shaped windows match an independent reference within two levels, with transparent margins untouched; dimming follows rounded corners if they exist by then.
- [ ] A focus change, a reload, and a new `_NET_WM_WINDOW_OPACITY` ease over the fade duration, sampled mid-transition, and repainting stops once they settle, back to the recorded idle baseline.
- [ ] Focus follows `_NET_ACTIVE_WINDOW` as today, and X input focus without it, including the root, `PointerRoot`, frames, and a window destroyed before its focus event is read.
- [ ] After warmup and 1,000 focus changes, XRes counts and owned pixmap bytes return to their settled values.
- [ ] Hardware records under at least two window managers with focus moved by the window manager itself, not by the probe, and benchmark scenes comparing inactive translucency with dimming, with Compust's and the X server's CPU.
- [ ] Formatting, strict Clippy, the full suite, the release build, and documentation in both languages, including the README's settings and rules tables and `compust.example.toml`, pass before the milestone is advertised as implemented.

### Ordered tasks

Each item is one reviewable issue, with the feature template's fields: **Problem or use case**, **Proposed behavior**, and **How to verify it**. The last field is the done criterion.

1. **Confirm the schema and semantics, [done](#task-1-done-schema-and-semantics).** **Problem or use case:** once 1.0 is out, a configuration field cannot change within 1.x. **Proposed behavior:** confirm the field names and ranges, that the focus settings replace the global opacity rather than multiply it, that changes ease over the fade duration, that fullscreen windows are exempt, and whether to follow X input focus. **How to verify it:** each open question in this section has a recorded decision.
2. **Ease opacity changes.** **Problem or use case:** a focus change, a reload, or a new application opacity jumps in one frame. **Proposed behavior:** a retargetable configured opacity per surface in `animation.rs` and `surface.rs`, with `scene.rs` painting until it settles. **How to verify it:** the unit tests and mid-transition pixel tests above pass, with existing focus and fade tests unchanged at `fade_ms = 0`.
3. **Add active and inactive opacity.** **Problem or use case:** dimming inactive windows takes a rule that must exclude menus, tooltips, and docks. **Proposed behavior:** `active_opacity` and `inactive_opacity` in `config.rs`, resolved with eligibility and precedence where `paint.rs` computes opacity. **How to verify it:** parsing, precedence, and eligibility tests pass, and frames with the default configuration do not change.
4. **Dim inactive windows.** **Problem or use case:** translucency costs a blur and occlusion for every inactive window. **Proposed behavior:** `inactive_dim` and a rule's `dim`, drawn in `paint.rs`, `gpu.rs`, and `crates/gl`, with the dimming in `damage.rs`. **How to verify it:** the dimming pixel tests pass in both painters.
5. **Follow focus without `_NET_ACTIVE_WINDOW`.** **Problem or use case:** under such window managers, nothing dims. **Proposed behavior:** X input focus as the fallback in `events.rs` and `atoms.rs`. **How to verify it:** the focus-source tests above pass, and the existing focus tests pass unchanged.
6. **Qualify and document.** **Problem or use case:** pixel tests alone do not show cost, real window-manager focus behavior, or resource stability. **Proposed behavior:** focus cycles in `tests/cases/`, hardware sessions under two window managers, benchmark runs, and updates to the README, the architecture, `compust.example.toml`, and both roadmaps. **How to verify it:** every acceptance item has evidence for the exact commit and environment.

### Task 1 done: schema and semantics

The owner confirmed on 2026-10-05: the fields are `active_opacity`, `inactive_opacity`, and `inactive_dim`, each 0–100, and a rule's `dim` in the same range; for eligible windows, `active_opacity` and `inactive_opacity` replace the global `opacity` rather than multiply it, so `opacity = 90` with `inactive_opacity = 80` shows an inactive window at 80%, and a rule's `opacity` still wins; opacity and dimming ease over the window's `fade_ms` rather than a duration of their own; fullscreen windows are exempt; and without `_NET_ACTIVE_WINDOW`, Compust follows X input focus.

### Decisions and later work

Dimming toward a color other than black, desaturating inactive windows, and focus per monitor are later work.

There are no delivery dates yet. Open an issue to discuss a bounded change or contribute an observed failure; avoid starting several overlapping backend designs before agreeing on the requirements.
