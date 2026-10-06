# Qtile on the Intel laptop with Xorg

Live checks of shadows, blur, transparency, and rounded corners passed under qtile with XRender and GL on 2026-10-05, from 23:18:17 to 23:20:07 -03:00, in the laptop's usual session. Compust was the packaged binary `0.3.0~beta.2+git20261005.3dfd920-1` from the Open Build Service repository, which qtile's autostart had launched; the repository was at the clean revision in [commit.txt](commit.txt), whose compositor source is that of `3dfd920`. The machine is an Intel Core i3-1005G1 with Iris Plus Graphics G1 (Ice Lake) and i915: Linux Mint 22.3, Linux 7.0.0-38-generic, Xorg 21.1.11 with modesetting and glamor, Mesa 25.2.8, and eDP-1 at 1366×768 and 60 Hz. Qtile 0.37.1 ran on its X11 backend with 3-pixel X borders on tiled windows, 2-pixel borders on floating ones, and 10-pixel margins.

The [configuration](compust.toml) is the session's own: XRender, vsync, 180 ms fades, blur radius 4, shadows of radius 10 offset by 5 and 5 at 90%, and corners of radius 8. The GL run changed only `backend`, by saving the file, and the original was restored afterward.

These checks do not use the repository's probe. [driver.py](tools/driver.py) moves to an empty qtile group, shows synthetic windows from [client.py](tools/client.py), captures the composed root with ImageMagick's `import`, and measures pixels. It then returns to the starting group. Checks allow three levels per channel.

## Results

| Check | XRender | GL |
| --- | --- | --- |
| Two tiled windows: border ring, interior, and outside of each rounded corner | PASS | PASS |
| Corners follow qtile's border colors when focus moves (`#7dcfff` and `#24283b`) | PASS | PASS |
| Shadow in the 10-pixel gap beside a tiled window, and its 5-pixel limit above | PASS | PASS |
| Floating dialogs keep their size above the tiles | PASS | PASS |
| 50% red window over white reads (255, 128, 128), its border half of qtile's color | PASS | PASS |
| Blur behind it over a 2-pixel checker, at the documented weight | PASS | PASS |
| Corners of the opaque and the 50% floating windows | PASS | PASS |
| Shadow of the opaque floating window on white: right, and limited on the left | PASS | PASS |
| Same frame after switching to another group and back | 0 pixels differ | 0 pixels differ |
| Fullscreen through qtile covers the screen, square corners included | PASS | PASS |
| Empty group after 32 rapid create, map, and destroy sequences | 0 pixels differ | 0 pixels differ |
| Compositor kept its process | PASS | PASS |

Each painter passed 20 checks of 20; [xrender/results.json](xrender/results.json) and [gl/results.json](gl/results.json) hold every measurement. Beside the opaque floating window, the shadow darkens white by 0.79 at the first pixel, 0.49 and 0.41 at the fifth and sixth, where half of the 0.90 maximum falls, and by 0.01 at the fourteenth; nothing remains from the fifteenth on. The 50% window's backdrop reads 32 and 96 in green around a flat 64: the blur flattens the checker, and the backdrop shows at the window's 50% over the sharp scene, as designed. Without blur it would read 0 and 128.

GL's captures differ from XRender's by at most two levels, below the bar: 3,048 pixels in the tiled scenes and 4,738 in the floating ones ([comparison.csv](gl/comparison.csv)). The fullscreen captures differ because that window alternates red and blue.

| Phase | Painter | Compust CPU | X server CPU | Compust RSS |
| --- | --- | ---: | ---: | ---: |
| Idle, four windows, 6 s | XRender | 0.0% | 0.3% | 3,712 KiB |
| Tiled window alternating red and blue at 60 Hz, 8 s | XRender | 1.1% | 6.2% | 3,712 KiB |
| Same, beneath a 600×400 window at 50% with blur, 8 s | XRender | 1.4% | 8.5% | 3,712 KiB |
| Idle, four windows, 6 s | GL | 0.2% | 0.2% | 78,336 KiB |
| Tiled window alternating red and blue at 60 Hz, 8 s | GL | 3.4% | 4.2% | 78,336 KiB |
| Same, beneath a 600×400 window at 50% with blur, 8 s | GL | 5.4% | 4.0% | 78,336 KiB |

CPU percentages use one core. The phases are short, ran once, and shared the session with a terminal, a bar, and the driver; no Present completions were counted.

## Observations

- After the reload back to XRender, Compust's RSS stayed at 77,176 KiB, against 3,712 KiB before GL was first selected. It was read by hand from `/proc` about ten seconds after the reload, and again five seconds later.
- The [compositor log](compust.log) names `Mesa Intel(R) UHD Graphics (ICL GT1)` for GL and records no fallback and no warning during either run. From 23:20:51 to 23:21:04, 44 seconds after the last run ended and with XRender restored, it logged `Present did not finish a submission; replacing its buffers` once a second, 14 times. Nothing was being tested then, and the cause was not found; DPMS reported the monitor on shortly afterward.
- An [earlier XRender pass](xrender-4px-checker/3-floating.png) used a checker of 4-pixel cells, which lay on the blur's 4-pixel grid. The pattern stayed visible behind the 50% window, with green from 24 to 104 around 64. That pass's [results](xrender-4px-checker/results.json) show three failures, all from the driver's expectations at the time, which the final driver corrects: the border color expected of a translucent window, where a shadow counts as ended, and the red expected behind the blur.

## Build and records

- Compust SHA-256 and package version: [binaries.txt](binaries.txt).
- [System](system.txt), [GPU and driver](gpu.txt), [kernel](kernel.txt), [outputs](outputs.txt), and [`compust --diagnose`](diagnose.txt).
- Captures and their hashes: [images-sha256.txt](images-sha256.txt).

The same evening, `cargo test --release --locked` passed 189 tests on this machine, and `tools/features-check.sh` passed its XRender rehearsal on Xvfb; neither is archived. The captures show the session's own bar and, in the margins, its wallpaper, besides the test windows. The log's home directory is abbreviated. These are fixed synthetic windows in one short session on one panel. Applications, Present timing, hotplug, and suspend and resume were not checked here.
