# Rounded corners on the Radeon Vega desktop with Xorg

The dedicated `--features` session passed with XRender and GL on 2026-10-05, from 14:47:57 to 14:48:45 -03:00, at the clean revision `1fc0a1c03cbae4b22a3e552b3642b2694cfcac3e` ([worktree.txt](features/worktree.txt) and [source.patch](features/source.patch) are empty). The machine is the AMD Ryzen 5 5600GT with integrated Radeon Vega graphics of the [earlier Xorg record](../features-vega-xorg/README.md): Linux 7.2.9-1-cachyos, Xorg 21.1.24 with modesetting and glamor, without TearFree, and HDMI-1 alone at 1920×1080 and 60 Hz. It used no window manager. The GL log names `AMD Radeon Graphics (radeonsi, renoir, ACO, DRM 3.64, 7.2.9-1-cachyos)` and contains no XRender fallback. The mouse was moved briefly during the run; the probe ignores input, and the captures do not include the cursor.

The [configuration](features/xrender/compust.toml) now blurs at radius 4 and rounds the corners scene's windows at radius 13. That scene shows an opaque rounded window with its rounded shadow, and a half-transparent one over a checkered pattern with its shadow and the blur behind it.

## Results

| Check | XRender | GL |
| --- | --- | --- |
| Shadow scene against independent reference | 2,073,600 pixels within 1 level | 2,073,600 pixels within 1 level |
| GL shadow scene against XRender | Reference | 45 pixels differ by 1 level |
| Both focus images against XRender | Reference | Identical |
| Corners scene: arcs leave what lies beneath uncolored | PASS | PASS |
| GL corners scene against XRender | Reference | 5,865 pixels differ by 1 level |
| Fullscreen suspension, popup resume, fresh recapture, cover removal | PASS | PASS |

| Painter and state | Compust CPU | X server CPU | Compust RSS before/after | X server RSS before/after |
| --- | --- | --- | --- | --- |
| XRender composed | 0.4% | 2.6% | 4,252 / 4,252 KiB | 91,548 / 91,548 KiB |
| XRender suspended | 0.0% | 1.1% | 4,252 / 4,252 KiB | 91,548 / 91,680 KiB |
| GL composed | 1.1% | 1.6% | 67,100 / 67,100 KiB | 91,672 / 91,672 KiB |
| GL suspended | 0.0% | 1.0% | 67,100 / 67,100 KiB | 91,672 / 91,672 KiB |

CPU percentages use one core over ten-second phases, after two seconds of warmup; one accounting tick is 0.1 percentage point.

## Build and records

- Compust SHA-256: `ca0c7c39f82c41b31c6afb6fd1c4707f3e0579e507f3352ffbd14c7033fb32af`.
- Probe SHA-256: `f8a439169ed3cf99d9ed6767639ee361e6e72b5157365e33b1bc999d835ff5ea`.
- [Session log](session.log), [XRender probe](features/xrender/probe.log), [GL probe](features/gl/probe.log), and [GL compositor](features/gl/compust.log).
- [Corners comparison](features/gl/corners-comparison.csv), [corners capture](features/gl/corners.png), and [XRender's](features/xrender/corners.png).

The CSV, configuration, metadata, and logs were copied unchanged. Captures are lossless PNG copies of the original PPM files, which [images-sha256.txt](images-sha256.txt) identifies with them. The archived X server log omits the kernel command line and raw EDID blocks and redacts monitor serial numbers.

The [first run](../corners-vega-xorg-overlap/README.md) of this scene, at `60e74db`, found a GL error that the fix in `1fc0a1c` corrects.
