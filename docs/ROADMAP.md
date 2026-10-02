# Roadmap

[English (US)](ROADMAP.md) | [Português (Brasil)](ROADMAP.pt-BR.md)

The goal is a minimal Rust compositor that becomes a practical choice for Xorg and XLibre users. “Better than picom” must eventually mean observable improvements in reliability, latency, resource use, or maintainability. Rewriting an existing feature in Rust is not, by itself, evidence of better performance.

## 0.1 foundation: implemented

The repository contains an executable compositor with XRender composition, fades, alpha transparency, convolution blur, shape clipping, application redraw tracking, stacking, window resize handling, wallpaper properties, and optional Present copy scheduling. The test suite checks real server behavior on Xvfb. Documentation and contribution paths are available in English and pt-BR.

This milestone establishes a base for experiments. It does not establish desktop-wide compatibility or hardware performance.

## Next: correctness across real desktops

Test Xorg and XLibre with actual window managers and Intel, AMD, and NVIDIA drivers. Record server, driver, configuration, and commit with every report. Cover reparenting after startup, rapid map/unmap/destroy sequences, decorated and override-redirect windows, menus, fullscreen transitions, wallpaper tools, and session shutdown.

Add real RandR resize and hotplug coverage, recovery tests for presentation failures, and resource accounting across repeated resizes. Check late creation of `WM_STATE`, large or off-screen shapes, and malformed application properties. Protocol requests can race with application destruction; fixes should narrow those races without hiding unrelated X errors.

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
