# Roadmap

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

The goal is a minimal Rust compositor that becomes a practical choice for Xorg and XLibre users. “Better than picom” must eventually mean observable improvements in reliability, latency, resource use, or maintainability. Rewriting an existing feature in Rust is not, by itself, evidence of better performance.

## 0.1 foundation: implemented

The repository contains an executable compositor with XRender composition, fades, alpha transparency, convolution blur, shape clipping, application redraw tracking, stacking, window resize handling, wallpaper properties, and optional Present copy scheduling. The test suite checks real server behavior on Xvfb. Documentation and contribution paths are available in English and pt-BR.

This milestone establishes a base for experiments. It does not establish desktop-wide compatibility or hardware performance.

## First beta: four steps

The first beta will use the existing XRender backend and declare support only for environments with recorded test evidence. It is intended for controlled community testing. The current 0.1.0 build remains an experimental prototype; defining these gates does not make it a beta release.

| Step | Status | Required result |
| --- | --- | --- |
| 1. Window stability | In progress | Window lifecycle, menus, fullscreen transitions, and invalid properties have reproducible coverage without crashes or stale/invisible windows. |
| 2. Monitors and resources | Pending | Resolution changes, monitor connection/disconnection, presentation recovery, and repeated resize resource use are verified. |
| 3. Real desktops | Pending | Xorg/XLibre sessions have recorded window-manager and driver coverage, plus CPU, memory, and frame-pacing measurements. |
| 4. Beta distribution | Pending | A versioned prerelease includes install/run instructions, known limits, verified artifacts, and a reproducible bug-report procedure. |

### 1. Window stability

Exercise rapid map/unmap/destroy sequences, interrupted fades, decorated windows, override-redirect menus, entering and leaving fullscreen, malformed properties, and windows disappearing during protocol requests. Start with deterministic Xvfb pixel regressions; record actual window-manager behavior in step 3.

**Acceptance:** each reproduced defect has a regression that fails without the fix; supported scenarios preserve the correct pixels and keep the compositor alive; the full suite, formatting, Clippy, and release build pass. Remaining scenarios stay explicitly open until exercised. Coverage below includes clients and frames, properties, rapid sequences, interrupted fades, extreme/off-screen shapes, and destruction before capture, shape, and geometry queries. Destruction between successive requests within a capture still needs deterministic reproduction.

### 2. Monitors and resources

Test RandR resolution changes and physical hotplug, supported presentation failure recovery, and repeated resizes with X-server resource accounting. Record the configuration and monitor arrangement used.

**Acceptance:** output recovers after each supported transition, presentation continues, and repeated operations do not produce unbounded growth in memory or server resources.

### 3. Real desktops

Run documented scenarios with real window managers on Xorg and XLibre. Record the server, window manager, GPU/driver, configuration, and exact commit. Measure idle and active CPU, memory, and frame pacing; fix failures in the environments proposed for beta support.

**Acceptance:** publish a compatibility matrix with evidence for each advertised environment, a reproducible measurement baseline, and remaining limitations. Untested driver/server combinations remain unqualified.

### 4. Beta distribution

Publish a versioned beta prerelease with build or binary installation instructions, configuration examples, startup/shutdown guidance, checksums for distributed artifacts, known limitations, and a bug-report template including diagnostics and reproduction steps.

**Acceptance:** a tester can install and run the exact release, return to their previous compositor, and report a failure from the supplied instructions. CI passes for the release commit and the previous three gates are satisfied for its declared support scope.

There is no beta date yet. These are four acceptance gates, not a fixed number of commits. GPU backend expansion and advanced effects can follow the first beta.

## Progress and verification

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

The full suite contains 44 tests: six unit, three CLI, and thirty-five X11 integration tests. All passed, along with formatting, Clippy with warnings treated as errors, and the release build. Step 1 remains in progress for destruction coverage between successive requests.

| Environment | Verified coverage | Evidence / limits |
| --- | --- | --- |
| Xvfb 21.1.24 on CachyOS, 320×240×24, XRender and Present 1.2 | Clients/frames, properties, rapid sequences, interrupted fades, shapes, and destruction with queued events; all 44 tests pass | Recorded 2026-10-02. New cases use `fade_ms = 0` or `1000`, `blur_radius = 0` or `4`; the border scenario uses `vsync = false`, the others use the default. Hierarchies are created directly, without real window-manager or GPU qualification. |
| Xorg with a real window manager and Intel/AMD/NVIDIA drivers | Pending | Requires a recorded server, window manager, driver, configuration, and commit. |
| XLibre with a real window manager and Intel/AMD/NVIDIA drivers | Pending | Requires the same environment evidence; Xvfb results do not establish support. |

Reproduce the lifecycle checks with the repository's pinned toolchain and Xvfb installed. The [CI runs](https://github.com/hashdefault/compust/actions/workflows/ci.yml) record results against each exact commit; include the revision printed below in local reports.

```sh
git rev-parse HEAD
cargo test --locked --test x11 client_lifecycle
cargo test --locked --test x11 properties
cargo test --locked --test x11 stability
cargo test --locked --test x11 fades
cargo test --locked --test x11 shapes
cargo test --locked --test x11 destruction
```

Set `XVFB=/path/to/Xvfb` when the server is outside `PATH`.

### Remaining acceptance work

Test Xorg and XLibre with actual window managers and Intel, AMD, and NVIDIA drivers. Record server, driver, configuration, and commit with every report. Cover reparenting after startup, rapid map/unmap/destroy sequences, decorated and override-redirect windows, menus, fullscreen transitions, wallpaper tools, and session shutdown.

Add real RandR resize and hotplug coverage, recovery tests for presentation failures, and resource accounting across repeated resizes. Extend destruction races into the gaps between pixmap naming, picture creation, and event subscription requests without hiding unrelated X errors. Preserve the large/off-screen shape and malformed-property coverage recorded above.

**Acceptance:** documented reproductions become regression tests when feasible; ordinary desktop activity does not crash or leave invisible/stale windows; repeated lifecycle changes do not grow server resources without bound. Maintain a compatibility matrix with evidence instead of a blanket “supported” label.

## Then: measure and reduce rendering work

Collect release-build baselines for idle CPU, application and X-server CPU, memory, frame pacing, and input-to-display latency. Compare equivalent scenes and effects against a recorded picom version/backend. Include high-resolution and mixed-refresh setups.

Introduce region-based damage, blur-region expansion, occlusion culling, and cached blur only where measurements justify the complexity. Evaluate multiple presentation buffers with explicit ownership and completion accounting.

**Acceptance:** a reproducible benchmark demonstrates the improvement, pixel tests remain correct at damaged-region boundaries, and idle work does not increase. Publish both the benefit and the workload where it disappears.

## Rendering backend and protocol expansion

Evaluate an EGL/OpenGL or Vulkan backend once the import and synchronization path is specified. Modern X11 integration may involve DRI3 DMA-BUF import, modifier negotiation, Present scheduling, and explicit synchronization. Each requires real implementation and driver testing; version probes do not count.

Color management, HDR, VRR, XLibre-specific extensions, and per-output scheduling require protocol and hardware investigations before support can be promised. Native Wayland is outside this roadmap.

**Acceptance:** a backend can import real application content without routine CPU readback, handles buffer lifetime correctly, recovers from supported failure modes, and has documented driver coverage and performance results.

## Everyday usability

Add a small, typed per-window rule system, configuration discovery and reload, clearer troubleshooting, and distribution packaging. Extend animations to movement and scale only after their interaction with input coordinates and window-manager geometry is settled. Consider shadows and rounded corners with proper shape and damage semantics.

**Acceptance:** behavior is configurable, documented in both languages, testable, and does not silently claim compatibility with picom's configuration or scripting language.

There are no delivery dates yet. Open an issue to discuss a bounded change or contribute an observed failure; avoid starting several overlapping backend designs before agreeing on the requirements.
