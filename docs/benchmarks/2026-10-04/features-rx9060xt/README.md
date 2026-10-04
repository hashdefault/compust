# Shadows, focus, and fullscreen suspension on the RX 9060 XT

The dedicated `--features` session passed with XRender and GL on 2026-10-04, from 18:58:02 to 18:58:50 -03:00. It used no window manager. The machine has an AMD Ryzen 5 5600X and Radeon RX 9060 XT, Linux 7.2.8-arch1-1, XLibre 25.1.9, modesetting with glamor and TearFree, and radeonsi. Only DP-2 was enabled, at 1920×1080, 60 Hz, depth 24. HDMI-1 was connected but disabled.

The GL log identifies `AMD Radeon RX 9060 XT (radeonsi, gfx1200, ACO, DRM 3.64, 7.2.8-arch1-1)` and contains no XRender fallback. Both compositor processes exited successfully after SIGTERM.

## Results

| Check | XRender | GL |
| --- | --- | --- |
| Shadow scene against independent reference | 2,073,600 pixels; 40 differ by at most 1 level | 2,073,600 pixels; 25 differ by at most 1 level |
| GL shadow scene against XRender | Reference | 45 pixels differ by at most 1 level |
| Both focus images against XRender | Reference | Identical |
| Focus switches, NONE, absent property | PASS | PASS |
| Composed phase, 10 seconds | 600 updates, 600 Present completions, 600 overlay Damage events | 600 updates, 600 Present completions, 600 overlay Damage events |
| Suspended phase, 10 seconds | 600 updates, zero Present completions and overlay Damage | 600 updates, zero Present completions and overlay Damage |
| Popup resume, fresh recapture, cover removal | PASS | PASS |

The independent reference and renderer comparison allow two levels per RGB channel. Every pixel in each full-screen shadow and focus capture was checked. Visual inspection also confirmed the shadow, both focus directions, and the green fullscreen content recaptured beneath the blue popup. An independent ImageMagick comparison of the two resumed images found a maximum channel difference of one level.

| Painter and state | Compust CPU | X server CPU | Compust RSS before/after | X server RSS before/after |
| --- | --- | --- | --- | --- |
| XRender composed | 0.3% | 1.9% | 4,236 / 4,236 KiB | 94,696 / 94,696 KiB |
| XRender suspended | 0.0% | 1.0% | 4,236 / 4,236 KiB | 94,696 / 94,696 KiB |
| GL composed | 0.6% | 1.4% | 71,112 / 71,112 KiB | 94,184 / 94,184 KiB |
| GL suspended | 0.0% | 1.1% | 71,112 / 71,112 KiB | 94,184 / 94,184 KiB |

CPU percentages use one core. Each phase had two seconds of warmup before measurement. Process accounting uses 100 ticks per second; a tick over ten seconds is 0.1 percentage point. The suspended measurements accumulated no Compust CPU ticks, which does not establish that its CPU use is exactly zero. This is one short synthetic workload, with a small dialog keeping the composed phase active; it is not a shadow performance benchmark or an endurance test.

## Build and records

The run used base revision `1969270257af37c092bad5d440d86ea74e680c01` plus the changes in [source.patch](features/source.patch), [source-sha256.txt](features/source-sha256.txt), and [worktree.txt](features/worktree.txt). The source and executable checksums were verified against the local files when collecting this record. This record identifies the worktree before these changes were committed.

- Compust SHA-256: `1df23c69d6b572ea331185994054cddc22321687e62cacd4aa603709c21844e6`.
- Probe SHA-256: `5cbe6be39cbd9bd0289152df26a573262b50aadf5a7ca0a19e2052e4342fe203`.
- [Session log](session.log), [XRender probe](features/xrender/probe.log), [GL probe](features/gl/probe.log), and [GL compositor](features/gl/compust.log).
- [XRender processes](features/xrender/processes.csv), [GL processes](features/gl/processes.csv), [XRender phases](features/xrender/fullscreen.csv), and [GL phases](features/gl/fullscreen.csv).
- [Shadow comparison](features/gl/shadows-comparison.csv), [left focus comparison](features/gl/focus-left-comparison.csv), and [right focus comparison](features/gl/focus-right-comparison.csv).
- [Shadow capture](features/gl/shadows.png), [left focus](features/gl/focus-left.png), [right focus](features/gl/focus-right.png), and [fullscreen resume](features/gl/fullscreen-resumed.png).

The CSV, configuration, metadata, source patch, and process logs were copied unchanged. Captures are archived as lossless PNG with the same RGB pixels; original PPM files remain in `artifacts/features-hardware`. [images-sha256.txt](images-sha256.txt) identifies both formats by their repository-relative paths. The archived X server log omits the kernel command line and raw EDID blocks and redacts monitor serial numbers; the original stays in the local artifact directory.

This record qualifies these fixtures on one AMD/XLibre display with both painters. Focus was controlled by the probe through `_NET_ACTIVE_WINDOW`; no window-manager focus behavior was tested here. Real fullscreen applications, other drivers and servers, multiple active monitors, shadow performance, and long-duration resource stability still require their own records.
