# Mixed-refresh monitor record: RX 9060 XT, 2026-10-04

All nine samples passed with Compust's current XRender and GL painters. The live Xmonad desktop used HDMI-1 at 1920×1080 60.000 Hz on the left and DP-2 at 1920×1080 179.998 Hz on the right, for a 3840×1080 root. Before this test both outputs were at 60 Hz; DP-2 was switched to 180 Hz. The test changed the primary output from DP-2 to HDMI-1 and back, without disconnecting either cable.

The machine has an AMD Ryzen 5 5600X and Radeon RX 9060 XT, Linux 7.2.8-arch1-1, XLibre 25.1.9, modesetting with amdgpu/radeonsi, Mesa 26.2.4, and Xmonad 0.18.1. The compositor and probe binaries are the recorded release builds from the source at `a3e6f3151847dc6dabd54bd6e21e2f38c250ed65`; no renderer or probe code changed. The worktree contained the benchmark-removal edits. [Binary hashes](binaries.txt), [source hashes](source-sha256.txt), [source patch](source.patch), [worktree state](worktree.txt), and the [three configurations](configs/) identify the run.

## Method

The existing `desktop_probe --hotplug` mode retained one managed window and one override-redirect marker during each primary-output transition. Every sample checked both colors near opposite corners of each monitor, both sides of their shared seam, and repainting the managed window in place. It then warmed up for two seconds and measured three idle seconds and three active seconds, with 180 requested marker updates at 60 updates per second.

The three profiles were XRender with Present, XRender with direct copying, and GL with Present. Fade and blur were disabled, opacity was 100%, and `max_fps` was 120. The GL [log](gl-present/compositor.log) confirms the RX 9060 XT renderer after each transition and has no XRender fallback. All three compositors stopped cleanly.

## Results

| Painter | Primary output | Active Present completions | Median / p95 interval | Compust / server CPU |
| --- | --- | ---: | ---: | ---: |
| GL | DP-2 | 271 | 11.111 / 16.669 ms | 1.33 / 2.33% |
| GL | HDMI-1 | 180 | 16.667 / 16.674 ms | 1.00 / 2.00% |
| GL | DP-2 restored | 271 | 11.111 / 16.669 ms | 1.00 / 2.67% |
| XRender | DP-2 | 274 | 11.111 / 16.669 ms | 0.67 / 2.67% |
| XRender | HDMI-1 | 180 | 16.667 / 16.674 ms | 0.33 / 2.33% |
| XRender | DP-2 restored | 271 | 11.111 / 16.668 ms | 0.33 / 3.00% |

The direct-copy samples had zero Present completions and 272, 271, and 271 active overlay Damage events. Pixel checks passed in every profile and topology. XRes reported 24,962,725 bytes of compositor-owned pixmaps in all nine samples; all resource counts stayed identical across the three samples within each profile.

Within the Present phases, the MSC/UST clock estimates followed the primary output: about 180 Hz on DP-2 and 60 Hz on HDMI-1. The largest active interval was 16.675 ms. The full [summary](summary.csv), per-sample `frames.csv`, `processes.csv`, `resources.csv`, and `topology.txt` preserve the raw observations.

## Limits and final desktop state

The ordinary desktop applications continued drawing: the idle Present phases contain 119–126 frames over three seconds, and active phases can contain more completions than the marker's 180 updates. These are functional monitor checks, not a controlled idle or independent throughput benchmark. CPU sampling uses 100 ticks per second; one tick in a three-second phase is about 0.33 percentage points of one core.

Compust composes the whole multi-monitor root as one surface. A 60-updates-per-second fixture and a 120-fps cap do not qualify independent 180-fps rendering on DP-2, per-output scheduling, input-to-panel latency, or tear-free scanout. This session does not repeat physical hotplug, run endurance tests, or exercise fade, blur, shadows, and fullscreen suspension across the mixed-refresh layout.

RSS was unchanged within each measured phase. XRender's RSS rose by 8 KiB between the first and final samples; GL's rose from 68,556 to 71,196 KiB across renderer recreations. These three transitions do not establish a long-term memory trend, and GL allocations are not included in XRes.

The original compositor binary and `target/shadows.toml` configuration were restored. Its SHA-256 remained `64ddd564d2717ca8121b341f2925d48c33dc667ed29ff952517cdcd67e7f7844`. DP-2 was left primary at 180 Hz and HDMI-1 at 60 Hz, matching the requested layout. [Before](outputs-before.txt), [test](outputs-60-180.txt), and [after](outputs-after.txt) output states are recorded.
