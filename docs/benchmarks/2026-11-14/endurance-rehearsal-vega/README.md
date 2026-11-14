# Rehearsal week of daily use on the Radeon Vega desktop

The `endurance_sample` example sampled the owner's daily sessions every ten minutes from 2026-11-06 14:42 to 2026-11-14 09:23 -03:00, while the released Compust 0.3.0-beta.4 composited them under Xmonad 0.18.1 on XLibre. The [desktop testing guide](../../../DESKTOP_TESTING.md#recorded-rehearsal-week-2026-11-06-to-2026-11-14) describes the machine, the results, and what they changed.

- [compust-endurance.csv](compust-endurance.csv): the 314 rows, as the sampler wrote them. SHA-256 `5d1cd230b515cf4c71e2904172b44b13f8a812d32507de2a2b176578d402eb22`.
- [summary.txt](summary.txt): the output of `endurance_sample summary --input compust-endurance.csv`, which exits with 1. The span, Present timeouts, and idle memory pass. The crash part fails because of a stop by hand on 2026-11-12 and a seven-minute session on 2026-11-10 whose only row came before Compust started; neither is a crash.

The sampler that wrote these rows was built at `a4fad80` and had no startup grace, so each session that it started at login opens with an `absent` row.
