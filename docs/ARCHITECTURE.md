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

`scene.rs` stores surfaces in server stacking order. `events.rs` updates their geometry, lifetime, shapes, and opacity. `surface/client.rs` discovers the application using `WM_STATE` through at most eight descendant levels. It subscribes to property and child-window events on each visited window before inspecting its state. Late creation or removal of `WM_STATE`, child creation, reparenting, and destruction refresh the affected frame's client association and opacity. The frame's opacity property takes precedence when present.

A descendant that disappears during discovery is skipped only for `BadWindow`. If the selected client disappears before its opacity can be read, the surviving frame uses its own opacity or the opaque default; subsequent lifecycle events refresh the association. Losing a client must not close the frame's surface. This is deliberately limited support for window-manager conventions, not complete EWMH/ICCCM implementation.

## Rendering and scheduling

The renderer paints wallpaper into a reusable root-depth buffer, composites windows from bottom to top, and uses an A8 mask for effective opacity. Per-pixel alpha remains part of the source picture. Shape rectangles clip both window painting and blur. The blur module uses two one-dimensional box convolutions, with an exact fixed-point coefficient sum of 65536.

`animation.rs` uses monotonic `Instant` values and integer smoothstep interpolation. Closing retargets from the sampled opacity so interrupted animations remain continuous. The event loop schedules a final frame at the endpoint, including when the duration is zero.

`compositor.rs` drains bounded event batches so an event storm cannot postpone painting forever. It repaints only after damage, relevant state changes, or an active animation, with `max_fps` as a ceiling. Idle operation polls the X socket with a one-second maximum wait to observe shutdown flags. Damage currently triggers a full-screen repaint; it is not yet a region optimization.

With Present, a single output buffer is submitted in COPY mode and reused only after both completion and idle notifications. Subscription IDs distinguish stale events after buffer recreation, and old subscriptions are released. This is a simple baseline, not a low-latency multi-buffer scheduler. Without Present, XRender copies directly to the overlay.

## Source map

| Area | Files |
| --- | --- |
| Startup and configuration | `main.rs`, `config.rs` |
| X11 ownership and capabilities | `session.rs`, `atoms.rs`, `capabilities.rs` |
| Captured resources and lifetime | `picture.rs`, `surface.rs` |
| Client/frame association | `surface/client.rs` |
| Scene and events | `scene.rs`, `events.rs`, `compositor.rs` |
| Effects and presentation | `animation.rs`, `renderer.rs`, `renderer/{paint,blur,present,wallpaper}.rs` |
| Real-server verification | `tests/x11.rs`, `tests/cases/`, `tests/support/` |

All source paths above are relative to `src/` except the test paths. The crate uses `unsafe_code = "forbid"`; that restriction covers this crate, not the internals of its dependencies. Rust prevents several memory errors, but protocol races, rendering mistakes, and resource leaks still require tests.

## Where to extend it

A GPU backend should preserve the scene and resource-lifetime contracts while replacing the rendering path. Do not add a generic backend abstraction until its actual import, synchronization, and presentation requirements are understood. DRI3 and Sync version probes currently provide diagnostics only.

The next optimization should be driven by profiles: damage regions must expand to include blur dependencies, and occlusion must account for translucent and shaped windows. Measure the X server as well as Compust because XRender delegates rendering work to it. The [roadmap](ROADMAP.md) defines acceptance criteria for these changes.
