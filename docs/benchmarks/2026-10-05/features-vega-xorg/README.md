# Shadows, focus, and fullscreen suspension on the Radeon Vega desktop with Xorg

The dedicated `--features` session passed with XRender and GL on 2026-10-05; [started.txt](features/started.txt) and [finished.txt](features/finished.txt) give its times. It used no window manager. The machine has an AMD Ryzen 5 5600GT with integrated Radeon Vega graphics (Cezanne), Linux 7.2.9-1-cachyos, Xorg 21.1.24 from CachyOS, modesetting with glamor, and radeonsi. Xorg 21.1.24's modesetting driver has no TearFree. Only HDMI-1 was enabled, at 1920×1080, 60 Hz, depth 24. DP-1 was connected but disabled.

The GL log identifies `AMD Radeon Graphics (radeonsi, renoir, ACO, DRM 3.64, 7.2.9-1-cachyos)` and contains no XRender fallback. Both compositor processes exited successfully after SIGTERM.

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

The independent reference and renderer comparison allow two levels per RGB channel. Every pixel in each full-screen shadow and focus capture was checked. The differing-pixel counts match those of the [RX 9060 XT record](../../2026-10-04/features-rx9060xt/README.md) on XLibre.

| Painter and state | Compust CPU | X server CPU | Compust RSS before/after | X server RSS before/after |
| --- | --- | --- | --- | --- |
| XRender composed | 0.5% | 3.4% | 4,372 / 4,372 KiB | 94,440 / 94,440 KiB |
| XRender suspended | 0.0% | 1.2% | 4,372 / 4,372 KiB | 94,440 / 94,440 KiB |
| GL composed | 1.1% | 1.7% | 66,180 / 66,180 KiB | 94,452 / 94,452 KiB |
| GL suspended | 0.0% | 1.3% | 66,180 / 66,180 KiB | 94,452 / 94,452 KiB |

CPU percentages use one core. Each phase had two seconds of warmup before measurement. Process accounting uses 100 ticks per second; a tick over ten seconds is 0.1 percentage point. The suspended measurements accumulated no Compust CPU ticks, which does not establish that its CPU use is exactly zero. This is one short synthetic workload, with a small dialog keeping the composed phase active.

## Build and records

The run used the clean revision `ed2cdb04f828887231660d4c10729245b4a80b73`: [worktree.txt](features/worktree.txt) and [source.patch](features/source.patch) are empty.

- Compust SHA-256: `cc8ad9e1e74afaf96b5dd76ccd93240c030e3abda24420507e9e76c630861a23`.
- Probe SHA-256: `5cbe6be39cbd9bd0289152df26a573262b50aadf5a7ca0a19e2052e4342fe203`.
- [Session log](session.log), [XRender probe](features/xrender/probe.log), [GL probe](features/gl/probe.log), and [GL compositor](features/gl/compust.log).
- [XRender processes](features/xrender/processes.csv), [GL processes](features/gl/processes.csv), [XRender phases](features/xrender/fullscreen.csv), and [GL phases](features/gl/fullscreen.csv).
- [Shadow comparison](features/gl/shadows-comparison.csv), [left focus comparison](features/gl/focus-left-comparison.csv), and [right focus comparison](features/gl/focus-right-comparison.csv).
- [Shadow capture](features/gl/shadows.png), [left focus](features/gl/focus-left.png), [right focus](features/gl/focus-right.png), and [fullscreen resume](features/gl/fullscreen-resumed.png).

The CSV, configuration, metadata, and process logs were copied unchanged. Captures are archived as lossless PNG with the same RGB pixels, which was checked for each image; original PPM files remain in `artifacts/vega-xorg-features`. [images-sha256.txt](images-sha256.txt) identifies both formats by their repository-relative paths. The archived X server log omits the kernel command line and raw EDID blocks and redacts monitor serial numbers; the original stays in the local artifact directory.

This record qualifies these fixtures on one AMD/Xorg display with both painters. Focus was controlled by the probe through `_NET_ACTIVE_WINDOW`; no window-manager focus behavior was tested here. Real fullscreen applications, other drivers, multiple active monitors, shadow performance, and long-duration resource stability still require their own records.
