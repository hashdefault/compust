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
| 1. Window stability | Complete on Xvfb | Window lifecycle, menus, fullscreen transitions, and invalid properties have reproducible coverage without crashes or stale/invisible windows. |
| 2. Monitors and resources | Automated checks complete; physical hotplug pending | Resolution changes, monitor connection/disconnection, presentation recovery, and repeated resize resource use are verified. |
| 3. Real desktops | Pending | Xorg/XLibre sessions have recorded window-manager and driver coverage, plus CPU, memory, and frame-pacing measurements. |
| 4. Beta distribution | Pending | A versioned prerelease includes install/run instructions, known limits, verified artifacts, and a reproducible bug-report procedure. |

### 1. Window stability

Exercise rapid map/unmap/destroy sequences, interrupted fades, decorated windows, override-redirect menus, entering and leaving fullscreen, malformed properties, and windows disappearing during protocol requests. Start with deterministic Xvfb pixel regressions; record actual window-manager behavior in step 3.

**Acceptance:** each reproduced defect has a regression that fails without the fix; supported scenarios preserve the correct pixels and keep the compositor alive; the full suite, formatting, Clippy, and release build pass. Coverage below includes clients and frames, properties, rapid sequences, interrupted fades, extreme/off-screen shapes, and destruction before and between capture requests. The automated Xvfb gate is complete; actual window-manager and driver qualification remains in step 3.

### 2. Monitors and resources

Test RandR resolution changes and physical hotplug, supported presentation failure recovery, and repeated resizes with X-server resource accounting. Record the configuration and monitor arrangement used.

**Acceptance:** output recovers after each supported transition, presentation continues, and repeated operations do not produce unbounded growth in memory or server resources.

The Xvfb scenarios below pass. Physical connector hotplug and multiple-monitor arrangements remain open; disabling a virtual CRTC does not establish those behaviors.

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

This batch brought the suite to 44 passing tests. The remaining capture-request boundaries are covered by the cases below.

### Step 1 complete: destruction between capture requests

Three tests in [capture_races.rs](../tests/cases/capture_races.rs) exercise eleven request boundaries: window attributes, geometry, pixmap naming, picture creation, client event subscription, `WM_STATE`, client tree discovery, shape event subscription, damage creation, shape rectangles, and opacity. A test-only loopback proxy pauses the selected request, completes a checked destruction through a separate Xvfb connection, then forwards the original bytes. Replies and events come from the real server; there are no production hooks or timing sleeps.

Every case waits for a surviving marker to render, checks the underlying window's pixels and compositor liveness, and probes the captured pixmap, picture, and damage identifiers to verify release. The existing capture recovery passed without production changes or broader error suppression. In temporary copies, disabling picture cleanup made the picture case fail with a still-allocated resource; disabling capture recovery made the compositor exit and the survivor case fail.

This completed step 1 with 47 passing tests: six unit, three CLI, and thirty-eight X11 integration tests. Formatting, strict Clippy, and the release build passed. Real desktops and physical hotplug still require their own evidence.

### Step 2: virtual monitor transitions, presentation recovery, and resources

Four tests in [monitors.rs](../tests/cases/monitors.rs) exercise real RandR requests with Present enabled and with direct XRender copying. The single Xvfb output is disabled, the root shrinks from 320×240 to 240×180, then the original size and CRTC configuration are restored. Sixteen measured cycles preserve the survivor's pixels, exact XRes resource counts, and full reported bytes of compositor-owned pixmaps. Separate cases disable and restore the output without changing root size. Disabling renderer replacement in a temporary copy makes both resize cases fail.

Four lifecycle tests in [resources.rs](../tests/cases/resources.rs) repeat mapped-window resizing and map/destroy sequences 32 times per rendering mode after warmup. Restored scenes retain exactly the same X-server resource counts and full reported pixmap bytes: 654,401 for the window scenarios and 878,401 for the larger monitor-test window. Disabling picture cleanup in a temporary copy makes all four lifecycle cases fail on their first measured cycle.

A fifth resource test holds extra pixmap references from another client. The full allocation total stays unchanged, while the old reference-weighted attribution drops from 641,066 to 429,600 bytes. The fixture uses XRes 1.2 `QueryResourceBytes` and verifies complete size coverage; `QueryClientPixmapBytes` is unsuitable for strict allocation comparisons while Present references change. Recorded Linux compositor RSS endpoints were unchanged or increased by one 4 KiB page. These finite checks detect growth in the exercised workloads; they do not establish a general memory bound or hardware GPU-memory usage.

Three tests in [presentation.rs](../tests/cases/presentation.rs) cover rejected submissions. A test proxy substitutes an incompatible pixmap in one Present request, producing a real server `BadMatch`. Previously the compositor exited. It now matches that exact submission, checks that its original back buffer and output still exist with compatible screen/depth, and continues through XRender for the rest of the session, including later root resizes. Invalid pixmap and window errors still terminate the compositor. Missing completion events and other presentation errors are outside this recovery policy.

The full suite now has 59 passing tests: six unit, three CLI, and fifty X11 integration tests. Formatting, strict Clippy, the release build, and documentation checks pass. Step 2 remains open for physical hotplug, multiple-monitor layouts, and hardware measurements. The automated configuration is one virtual output at 320×240, temporarily 240×180, with `fade_ms = 0`, `blur_radius = 0`, and each vsync mode.

| Environment | Verified coverage | Evidence / limits |
| --- | --- | --- |
| Xvfb 21.1.24 on CachyOS, 320×240×24, XRender and Present 1.2 | Clients/frames, properties, rapid sequences, interrupted fades, shapes, queued destruction events, and eleven capture-request boundaries; all 47 tests pass | Recorded 2026-10-02. Capture races use `fade_ms = 0`, `blur_radius = 0`, and default vsync. Earlier fade/shape cases also use `fade_ms = 1000`, `blur_radius = 4`, or `vsync = false` as described above. Hierarchies are created directly, without real window-manager or GPU qualification. |
| Same Xvfb, one virtual output, RandR and XRes | Root shrink/restore, CRTC disable/restore, rejected Present submission, and repeated resource accounting; all 59 tests pass | Recorded 2026-10-02 (local time). Server resource counts and bytes are checked at matching rendered states; physical hotplug and multiple monitors remain unverified. |
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
cargo test --locked --test x11 capture_races
cargo test --locked --test x11 monitors
cargo test --locked --test x11 resources
cargo test --locked --test x11 presentation
```

Set `XVFB=/path/to/Xvfb` when the server is outside `PATH`.

### Remaining acceptance work

Test Xorg and XLibre with actual window managers and Intel, AMD, and NVIDIA drivers. Record server, driver, configuration, and commit with every report. Cover reparenting after startup, rapid map/unmap/destroy sequences, decorated and override-redirect windows, menus, fullscreen transitions, wallpaper tools, and session shutdown.

Exercise physical monitor hotplug and multiple-monitor layouts on real hardware, and measure longer-running memory and presentation behavior. The virtual RandR transitions, narrow Present rejection recovery, and repeated XRes accounting above are complete. Preserve the capture-request destruction, resource cleanup, large/off-screen shape, and malformed-property coverage recorded above.

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
