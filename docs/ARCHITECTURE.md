# Architecture

[English (US)](ARCHITECTURE.md) | [Português (Brasil)](ARCHITECTURE.pt-BR.md)

Compust is an X11 client that owns the compositor selection for one screen. The X server retains responsibility for input, application connections, and display hardware. The window manager retains responsibility for placement, decorations, focus, and workspaces.

## From a window to a frame

```text
Application draws into a redirected window
                  |
          Damage / window events
                  |
       Scene: stacking and lifetime
                  |
   XRender: wallpaper -> blur -> windows
                  |
       Present copy or XRender copy
                  |
     Composite overlay (empty input region)
```

`session.rs` negotiates capabilities and claims `_NET_WM_CM_Sn`. A short server grab makes checking the existing owner and acquiring redirection atomic with respect to other clients. The grab is released on the error path too. A real server timestamp is obtained before claiming the selection and announcing `MANAGER`.

`surface.rs` captures each viewable top-level window using a named pixmap. Damage is attached to the pixmap, which remains valid while the closing animation completes even if the original window disappears. `picture.rs` owns XRender pictures and owned pixmaps; closing the final connection also releases server resources and redirection after startup failures.

`scene.rs` stores surfaces in server stacking order. It mirrors the root's children from create, destroy, reparent, configure, and circulate events, so restacking costs no `QueryTree` round trip. A query's reply already reflects every event the server generated before handling it, so events with an earlier sequence number leave the mirror alone; an event that names a window the mirror lacks triggers a new query. Destroyed windows' surfaces remain below their former upper neighbor until the fade finishes. Remapping a window replaces its content only after successful capture, preserving the current opacity for reopening. `events.rs` updates their geometry, lifetime, shapes, and opacity. The server gives a window a new pixmap on every resize and reports every resize, so a surface is recaptured when the size in a `ConfigureNotify` differs from the captured one. That includes a window resized and restored before the events are handled, whose first event carries the intermediate size; comparing only the current size would miss it. A move only updates the position from the event, without a round trip. `surface/client.rs` discovers the application using `WM_STATE` through at most eight descendant levels. It subscribes to property and child-window events on each visited window before inspecting its state. Late creation or removal of `WM_STATE`, child creation, reparenting, and destruction refresh the affected frame's client association and opacity. The frame's opacity property takes precedence when present.

A descendant that disappears during discovery is skipped only for `BadWindow`. If the selected client disappears before its opacity can be read, the surviving frame uses its own opacity or the opaque default; subsequent lifecycle events refresh the association. Losing a client must not close the frame's surface. This is deliberately limited support for window-manager conventions, not complete EWMH/ICCCM implementation.

Property reads validate the wire representation before using application data. `WM_STATE` must have type `WM_STATE`, format 32, and exactly two values. `_NET_WM_WINDOW_OPACITY` must contain one 32-bit `CARDINAL`, with no extra data left in the property. Invalid values are treated as absent, including fallback from malformed frame opacity to valid client opacity. Ordinary X request errors still follow the existing error paths.

## Rendering and scheduling

The renderer paints wallpaper into a reusable root-depth buffer, composites windows from bottom to top, and uses an A8 mask for effective opacity. Per-pixel alpha remains part of the source picture. Shape rectangles clip both window painting and blur. They are moved to root coordinates in 32-bit arithmetic and limited to the window and the repaint area before they reach the server, so extreme off-screen shapes cannot overflow 16-bit coordinates. The blur module builds a pyramid around each translucent window: each level halves the previous one with bilinear sampling, then the coarsest level is scaled back up, level by level, into a backdrop buffer shown within the window's shape. The radius selects the depth, rounded to 2, 4, 8, or 16 pixels. The pyramid area adds a margin of twice the coarsest step and aligns to that step, so pixels left from earlier frames cannot reach the window and the result stays anchored to the screen as windows move. Level buffers exist only when blur is enabled. Glamor-based drivers accelerate these bilinear transforms but render convolution filters, used by an earlier box blur, on the CPU; the [desktop qualification guide](DESKTOP_TESTING.md#recorded-pyramid-blur-session-2026-10-03) compares both.

`animation.rs` uses monotonic `Instant` values and integer smoothstep interpolation. Closing and reopening retarget from the sampled opacity so interrupted animations remain continuous. The event loop schedules a final frame at the endpoint, including when the duration is zero.

Each frame repaints only an area of the root. A window's Damage reports the bounding box of its changes since the previous report, which locates them without fetching the region. `renderer/damage.rs` keeps what the last frame showed of each surface: its bounds, shape, captured picture, effective opacity, and stacking position. Each change also records the lowest layer of the stack it alters; the background is layer 0. A surface's content changes from its own layer up, and so do the old and new bounds of a surface that moved, was resized or recaptured, changed shape or opacity, appeared, or left. A restack changes the scene beneath each surface that swapped places, across both surfaces' bounds, but the output only where they overlap. An exposure of the overlay changes the output alone; a new renderer or a wallpaper change starts from the background, and a Present rejection repaints everything. `region.rs` keeps the repaint area as at most 16 rectangles, merging the pair whose hull adds the least, so it can cover more than changed but never less. Painting is clipped to the area, and a frame whose area is empty is not painted at all.

Each blurred surface keeps its blurred backdrop in a buffer that covers its bounds on the screen. Blur reads around every pixel it writes, so a change beneath a surface anywhere in its footprint blurs it again, which repaints the whole footprint and changes the surface for those above it. A surface repainted where its kept backdrop no longer matches its bounds also blurs again. Any other repaint, such as the surface's own content changing, its opacity fading, or a window above it changing, copies the kept backdrop and costs no blur. Buffers round up to 64 pixels and are replaced only when too small or over four times too large; a buffer goes when its surface stops being blurred.

`renderer/cover.rs` skips what opaque surfaces hide. A surface without an alpha channel, shown at full opacity, hides its shape from every surface beneath it and from the background, which are painted only where the area remains in sight. A surface due to blur again reads the scene beneath it across its footprint, so nothing there is hidden from the surfaces beneath it. A blurred surface hidden whole by those above it does not blur at all; it drops its kept backdrop instead, so that it blurs again once something uncovers it. The covered area is kept as at most 64 rectangles, past which it hides less than it could, never more.

`compositor.rs` drains bounded event batches so an event storm cannot postpone painting forever. It repaints only after damage, relevant state changes, or an active animation, with `max_fps` as a ceiling. An event that changes neither a shown surface nor their order does not repaint: with one Present buffer, such a frame would hold the next real one back by a vblank. Idle operation polls the X socket with a one-second maximum wait to observe shutdown and reload flags.

`config.rs` reads the `--config` file, or else the first `compust/compust.toml` in the XDG configuration directories. SIGUSR1 sets a flag that the event loop checks after each event batch. The reload repeats that search, so it reads what a restart would; a file that cannot be read or is invalid leaves the running configuration untouched. Opacity and the frame ceiling are read on every frame. Each fade transition takes the duration configured when it starts, so a reloaded `fade_ms` also governs windows opened earlier. The renderer records the blur radius and vsync setting it was built for, and a reload that changes either replaces it at the next paint, after Present has released the buffer as for any frame. The signal handlers are installed before the compositor selection is claimed: a client can signal the process as soon as it sees the selection, and SIGUSR1 would otherwise terminate it. SIGUSR1 follows picom; SIGHUP is avoided because a closing terminal sends it.

With Present, a single output buffer is submitted in COPY mode with the repaint area as its update region, so the server copies only that area; the buffer keeps the rest of the previous frame, which still matches the screen. It is reused only after both completion and idle notifications. A RandR screen change is the exception: the renderer is replaced without waiting, because a CRTC reconfiguration on modesetting/amdgpu hardware discarded both events for a pending submission. A one-second timeout covers losses from any other cause: the renderer is replaced and the frame repainted once. If the next submission also times out, the buffers are replaced again but painting waits for new damage, so a server that stops reporting does not cause a repaint every second. A root `ConfigureNotify` can request that replacement but not cancel it. Subscription IDs distinguish stale events after buffer recreation, and old subscriptions are released. This is a simple baseline, not a low-latency multi-buffer scheduler. Without Present, XRender copies the area directly to the overlay.

A `BadMatch` on the exact outstanding PresentPixmap request can fall back to XRender after checked geometry queries confirm that the original back pixmap and output still exist on the same screen with matching depth and expected buffer dimensions. The rejected request does not own the buffer, so the idle/completion wait is cleared. The session then marks Present unavailable, so the fallback survives a root resize or a configuration reload that recreates the renderer. A warning records the rejected request. Invalid resources and unrelated errors are not covered by this recovery.

## Source map

| Area | Files |
| --- | --- |
| Startup and configuration | `main.rs`, `config.rs` |
| X11 ownership and capabilities | `session.rs`, `atoms.rs`, `capabilities.rs` |
| Captured resources and lifetime | `picture.rs`, `surface.rs` |
| Client/frame association | `surface/client.rs` |
| Scene and events | `scene.rs`, `events.rs`, `compositor.rs` |
| Effects and presentation | `animation.rs`, `region.rs`, `renderer.rs`, `renderer/{paint,damage,cover,blur,present,wallpaper}.rs` |
| Real-server verification | `tests/x11.rs`, `tests/cases/`, `tests/support/` |

All source paths above are relative to `src/` except the test paths. The crate uses `unsafe_code = "forbid"`; that restriction covers this crate, not the internals of its dependencies. Rust prevents several memory errors, but protocol races, rendering mistakes, and resource leaks still require tests.

## Capture verification

Capture-race tests route only the compositor connection through `tests/support/proxy.rs`. The proxy frames native-endian X11 requests with x11rb, forwards replies and events unchanged, and joins its worker on shutdown. `tests/support/capture.rs` discovers extension opcodes from Xvfb and completes a checked window destruction immediately before forwarding the selected request. The test client retains a direct server connection for pixel checks and resource probes. This transport belongs only to the test binary; normal compositor execution has no interception hooks.

The same transport supports presentation tests by substituting a request field before forwarding it; Xvfb generates the actual error. It also records the update region of each PresentPixmap request, so region tests can check how much of the screen each frame replaced. Substituting a never-triggered SYNC wait fence withholds a submission's completion and idle events instead. `tests/support/monitors.rs` uses checked RandR requests on that test's isolated server. `tests/support/resources.rs` queries XRes using the compositor selection owner's window as the client identifier. It compares resource counts and the full reported bytes of owned pixmap XIDs at matching rendered states after warmup. XRes 1.2 `QueryResourceBytes` provides these sizes without dividing by reference counts; `QueryClientPixmapBytes` attribution can change while Present holds a reference. The fixture requires size entries for every owned pixmap and excludes cross references to avoid duplicate accounting. XRes is a development-only feature; the production compositor does not require the extension. Test screenshots use the current overlay dimensions.

## Where to extend it

A GPU backend should preserve the scene and resource-lifetime contracts while replacing the rendering path. Do not add a generic backend abstraction until its actual import, synchronization, and presentation requirements are understood. DRI3 and Sync version probes currently provide diagnostics only.

The next optimization should be driven by profiles. Every saved repaint adds state that a missed change leaves stale, so each needs a test that makes that change and compares the frame with one from a fresh renderer. Measure the X server as well as Compust because XRender delegates rendering work to it. The [roadmap](ROADMAP.md) defines acceptance criteria for these changes.
