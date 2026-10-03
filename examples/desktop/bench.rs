use super::{
    Args, Surface,
    metrics::{Delta, Process, Snapshot},
    topology,
};
use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use x11rb::{
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT,
    connection::Connection,
    protocol::{
        Event,
        present::CompleteKind,
        xproto::{
            AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, PropMode,
            WindowClass,
        },
    },
    wrapper::ConnectionExt as _,
};

const BACKDROP: u32 = 0x0040_4040;
const RED: u32 = 0x00ff_0000;
const BLUE: u32 = 0x0000_00ff;
const WARMUP: Duration = Duration::from_secs(2);
/// Updates per second requested by the animated scenes.
const RATE: f64 = 60.0;
/// Windows opened and closed by the open-close scene.
const CYCLES: u32 = 100;

/// A fixed workload. Every window is override-redirect, so the geometry is the same under
/// any window manager, and a backdrop covers the rest of the screen.
#[derive(Clone, Copy, ValueEnum)]
pub(super) enum Scene {
    /// Only the backdrop; nothing changes.
    Idle,
    /// A 64×64 window alternates colors 60 times per second.
    SmallUpdate,
    /// A half-transparent window covering the screen alternates colors 60 times per second.
    FullscreenTranslucent,
    /// Eight overlapping half-transparent windows; the top one alternates colors.
    EightTranslucent,
    /// A window moves and resizes 60 times per second.
    MoveResize,
    /// Windows open and close in turn; each waits for the first frame that shows the change.
    OpenClose,
}

impl Scene {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::SmallUpdate => "small-update",
            Self::FullscreenTranslucent => "fullscreen-translucent",
            Self::EightTranslucent => "eight-translucent",
            Self::MoveResize => "move-resize",
            Self::OpenClose => "open-close",
        }
    }
}

#[derive(Clone, Copy)]
struct Area {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

impl Area {
    fn center(self) -> (i16, i16) {
        (
            self.x.saturating_add_unsigned(self.width / 2),
            self.y.saturating_add_unsigned(self.height / 2),
        )
    }
}

/// One update of an animated scene, given the update's index.
type Step<'a> = Box<dyn FnMut(u32) -> Result<()> + 'a>;

/// What one measured interval saw, with process snapshots at its ends.
struct Measured {
    observed: Observed,
    before: Snapshot,
    after: Snapshot,
    elapsed: Duration,
}

/// Present completions and overlay Damage seen while measuring.
#[derive(Default)]
struct Observed {
    /// UST in microseconds and MSC of each completed compositor presentation.
    frames: Vec<(u64, u64)>,
    damage: u32,
    gpu: Option<GpuBusy>,
}

/// Samples of a GPU load percentage, such as amdgpu's `gpu_busy_percent`, ten times a second.
struct GpuBusy {
    path: PathBuf,
    next: Instant,
    samples: Vec<u8>,
}

const GPU_PERIOD: Duration = Duration::from_millis(100);

impl Observed {
    fn measuring(gpu: Option<&Path>) -> Self {
        Self {
            gpu: gpu.map(|path| GpuBusy {
                path: path.to_owned(),
                next: Instant::now(),
                samples: Vec::new(),
            }),
            ..Self::default()
        }
    }

    fn sample(&mut self) -> Result<()> {
        if let Some(gpu) = &mut self.gpu {
            let now = Instant::now();
            if now >= gpu.next {
                let text = std::fs::read_to_string(&gpu.path)
                    .with_context(|| format!("reading {}", gpu.path.display()))?;
                gpu.samples.push(text.trim().parse()?);
                gpu.next = (gpu.next + GPU_PERIOD).max(now);
            }
        }
        Ok(())
    }

    /// When to wake for the next GPU sample, if sampling.
    fn wake(&self, deadline: Instant) -> Instant {
        self.gpu
            .as_ref()
            .map_or(deadline, |gpu| gpu.next.min(deadline))
    }

    fn record(&mut self, surface: &Surface, event: &Event) {
        match event {
            Event::DamageNotify(event) if event.drawable == surface.overlay => self.damage += 1,
            Event::PresentCompleteNotify(event)
                if event.event == surface.present && event.kind == CompleteKind::PIXMAP =>
            {
                self.frames.push((event.ust, event.msc));
            }
            _ => {}
        }
    }

    fn drain(&mut self, surface: &Surface) -> Result<()> {
        while let Some(event) = surface.poll_event()? {
            self.record(surface, &event);
        }
        Ok(())
    }
}

/// Set up `scene` over a backdrop, warm up, and measure it for `args.seconds`.
pub(super) fn run(
    surface: &Surface,
    args: &Args,
    scene: Scene,
    processes: &[Process],
) -> Result<()> {
    topology::capture(surface, &args.output)?;
    let screen = Area {
        x: 0,
        y: 0,
        width: surface.width,
        height: surface.height,
    };
    window(surface, screen, BACKDROP, None)?;
    wait_pixel(surface, screen.center(), |pixel| pixel == BACKDROP, None)?;
    let seconds = Duration::from_secs(u64::from(args.seconds));
    let measure = |step| measure(surface, processes, args.gpu_busy.as_deref(), seconds, step);
    let (measured, latencies) = match scene {
        Scene::Idle => (measure(None)?, None),
        Scene::SmallUpdate => {
            let area = Area {
                x: 200,
                y: 200,
                width: 64,
                height: 64,
            };
            let target = window(surface, area, RED, None)?;
            wait_pixel(surface, area.center(), |pixel| pixel == RED, None)?;
            (measure(Some(alternate(surface, target)))?, None)
        }
        Scene::FullscreenTranslucent => {
            let target = window(surface, screen, RED, Some(50))?;
            wait_pixel(surface, screen.center(), half_red, None)?;
            (measure(Some(alternate(surface, target)))?, None)
        }
        Scene::EightTranslucent => {
            // Cascaded so that the top window's lower right part lies over the backdrop
            // alone; the stack fits a 1366×768 screen.
            let mut target = 0;
            for index in 0..8_i16 {
                let area = Area {
                    x: 48 + index * 48,
                    y: 40 + index * 36,
                    width: 480,
                    height: 360,
                };
                let color = if index == 7 { RED } else { 0x0000_c000 };
                target = window(surface, area, color, Some(50))?;
            }
            wait_pixel(surface, (840, 500), half_red, None)?;
            (measure(Some(alternate(surface, target)))?, None)
        }
        Scene::MoveResize => {
            let start = Area {
                x: 100,
                y: 80,
                width: 240,
                height: 180,
            };
            let target = window(surface, start, RED, None)?;
            wait_pixel(surface, start.center(), |pixel| pixel == RED, None)?;
            let steps: Step<'_> = Box::new(move |frame| {
                let step = i32::try_from(frame % 600)?;
                // Wander over a 1366×768 screen and change both dimensions every frame.
                let aux = ConfigureWindowAux::new()
                    .x(100 + step * 7 % 600)
                    .y(80 + step * 5 % 300)
                    .width(u32::try_from(240 + step * 3 % 120)?)
                    .height(u32::try_from(180 + step * 2 % 90)?);
                surface.conn.configure_window(target, &aux)?;
                surface.conn.flush()?;
                Ok(())
            });
            (measure(Some(steps))?, None)
        }
        Scene::OpenClose => {
            let (measured, latencies) = open_close(surface, processes, args.gpu_busy.as_deref())?;
            (measured, Some(latencies))
        }
    };
    write(args, scene, &measured, latencies.as_deref())?;
    topology::resources(surface, &args.output, false)?;
    println!(
        "PASS scene={} seconds={:.3} completions={} damage_events={}",
        scene.name(),
        measured.elapsed.as_secs_f64(),
        measured.observed.frames.len(),
        measured.observed.damage
    );
    Ok(())
}

/// Create and map an override-redirect window, with an opacity percentage if translucent.
fn window(surface: &Surface, area: Area, color: u32, opacity: Option<u8>) -> Result<u32> {
    let window = create(surface, area, color)?;
    if let Some(percent) = opacity {
        let value = u32::try_from(u64::from(u32::MAX) * u64::from(percent) / 100)?;
        surface
            .conn
            .change_property32(
                PropMode::REPLACE,
                window,
                surface.atom("_NET_WM_WINDOW_OPACITY")?,
                AtomEnum::CARDINAL,
                &[value],
            )?
            .check()?;
    }
    surface.conn.map_window(window)?.check()?;
    Ok(window)
}

fn create(surface: &Surface, area: Area, color: u32) -> Result<u32> {
    let window = surface.conn.generate_id()?;
    surface
        .conn
        .create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            surface.root,
            area.x,
            area.y,
            area.width,
            area.height,
            0,
            WindowClass::INPUT_OUTPUT,
            COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .background_pixel(color)
                .override_redirect(1),
        )?
        .check()?;
    surface
        .conn
        .change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"Compust benchmark",
        )?
        .check()?;
    Ok(window)
}

/// Paint `target` red and blue in turn, one color per update.
fn alternate(surface: &Surface, target: u32) -> Step<'_> {
    Box::new(move |frame| surface.paint(target, if frame.is_multiple_of(2) { BLUE } else { RED }))
}

/// Red at half opacity over the backdrop, allowing for the compositors' 8-bit alpha.
fn half_red(pixel: u32) -> bool {
    let channel = |shift: u32| i32::try_from((pixel >> shift) & 0xff).unwrap_or(-1);
    let expected = [(16, 0x9f), (8, 0x20), (0, 0x20)];
    expected
        .iter()
        .all(|&(shift, value)| (channel(shift) - value).abs() <= 3)
}

/// Wait until the composited pixel at `point` satisfies `accept`, recording frames if asked.
fn wait_pixel(
    surface: &Surface,
    point: (i16, i16),
    accept: impl Fn(u32) -> bool,
    mut observed: Option<&mut Observed>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        surface.conn.flush()?;
        let pixel = surface.pixel(point)?;
        if accept(pixel) {
            return Ok(());
        }
        let mut event = surface.poll_event()?;
        while event.is_none() {
            ensure!(
                Instant::now() < deadline,
                "timed out: pixel at {point:?} remained {pixel:06x}"
            );
            match observed.as_deref_mut() {
                Some(observed) => {
                    observed.sample()?;
                    surface.wait(observed.wake(deadline))?;
                }
                None => surface.wait(deadline)?,
            }
            event = surface.poll_event()?;
        }
        while let Some(current) = event {
            if let Some(observed) = observed.as_deref_mut() {
                observed.record(surface, &current);
            }
            event = surface.poll_event()?;
        }
    }
}

/// Run `step`, if any, 60 times per second through a warmup and then `duration`, measuring
/// the latter.
fn measure(
    surface: &Surface,
    processes: &[Process],
    gpu: Option<&Path>,
    duration: Duration,
    mut step: Option<Step<'_>>,
) -> Result<Measured> {
    let mut frame = 0;
    let mut warmup = Observed::default();
    animate(surface, WARMUP, &mut frame, step.as_mut(), &mut warmup)?;
    let before = Snapshot::read(processes)?;
    let started = Instant::now();
    let mut observed = Observed::measuring(gpu);
    animate(surface, duration, &mut frame, step.as_mut(), &mut observed)?;
    let elapsed = started.elapsed();
    Ok(Measured {
        observed,
        before,
        after: Snapshot::read(processes)?,
        elapsed,
    })
}

fn animate(
    surface: &Surface,
    duration: Duration,
    frame: &mut u32,
    mut step: Option<&mut Step<'_>>,
    observed: &mut Observed,
) -> Result<()> {
    let start = Instant::now();
    let end = start + duration;
    let mut updates = 0_u32;
    loop {
        observed.drain(surface)?;
        observed.sample()?;
        let now = Instant::now();
        if now >= end {
            return Ok(());
        }
        let next = start + Duration::from_secs_f64(f64::from(updates) / RATE);
        match step.as_deref_mut() {
            Some(step) if now >= next => {
                step(*frame)?;
                *frame = frame.wrapping_add(1);
                updates += 1;
            }
            Some(_) => surface.wait(observed.wake(next.min(end)))?,
            None => surface.wait(observed.wake(end))?,
        }
    }
}

/// Open and close `CYCLES` windows, timing each change until the overlay shows it.
fn open_close(
    surface: &Surface,
    processes: &[Process],
    gpu: Option<&Path>,
) -> Result<(Measured, Vec<(Duration, Duration)>)> {
    let area = Area {
        x: 300,
        y: 200,
        width: 320,
        height: 240,
    };
    let point = area.center();
    let mut warmup = Observed::default();
    for cycle in 0..4 {
        cycle_window(surface, area, point, cycle, &mut warmup)?;
    }
    let before = Snapshot::read(processes)?;
    let started = Instant::now();
    let mut observed = Observed::measuring(gpu);
    let mut latencies = Vec::new();
    for cycle in 0..CYCLES {
        latencies.push(cycle_window(surface, area, point, cycle, &mut observed)?);
    }
    let elapsed = started.elapsed();
    let measured = Measured {
        observed,
        before,
        after: Snapshot::read(processes)?,
        elapsed,
    };
    Ok((measured, latencies))
}

fn cycle_window(
    surface: &Surface,
    area: Area,
    point: (i16, i16),
    cycle: u32,
    observed: &mut Observed,
) -> Result<(Duration, Duration)> {
    let color = if cycle.is_multiple_of(2) { RED } else { BLUE };
    let window = create(surface, area, color)?;
    let opened = Instant::now();
    surface.conn.map_window(window)?;
    wait_pixel(surface, point, |pixel| pixel == color, Some(observed))?;
    let open = opened.elapsed();
    let closed = Instant::now();
    surface.conn.destroy_window(window)?;
    wait_pixel(surface, point, |pixel| pixel == BACKDROP, Some(observed))?;
    Ok((open, closed.elapsed()))
}

/// Vblanks without a completion between consecutive frames, from MSC. Present can switch the
/// CRTC it follows on a multi-monitor root, and the new CRTC's counter has another base. The
/// vblank period is the median of each pair's time per MSC step; a step that disagrees with
/// its pair's time by more than a quarter, and by more than two vblanks, is counted as a
/// discontinuity, and its skipped vblanks are estimated from the time instead.
fn skipped_vblanks(frames: &[(u64, u64)]) -> (u64, u32) {
    let gap = |pair: &[(u64, u64)]| match pair {
        [(ust0, msc0), (ust1, msc1)] => {
            Some((ust1.saturating_sub(*ust0), msc1.wrapping_sub(*msc0)))
        }
        _ => None,
    };
    let mut periods: Vec<u64> = frames
        .windows(2)
        .filter_map(gap)
        .filter_map(|(time, step)| time.checked_div(step))
        .collect();
    periods.sort_unstable();
    let period = periods
        .get(periods.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(0);
    let (mut skipped, mut discontinuities) = (0_u64, 0_u32);
    for (time, step) in frames.windows(2).filter_map(gap) {
        let by_time = time
            .saturating_add(period / 2)
            .checked_div(period)
            .unwrap_or(step)
            .max(1);
        if step >= 1 && step.abs_diff(by_time) <= (by_time / 4).max(2) {
            skipped += step - 1;
        } else {
            discontinuities += 1;
            skipped += by_time - 1;
        }
    }
    (skipped, discontinuities)
}

/// Nearest-rank percentile of sorted values.
fn percentile(sorted: &[f64], percent: usize) -> Option<f64> {
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted.get(rank.checked_sub(1)?).copied()
}

fn milliseconds(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| format!("{value:.3}"))
}

fn write(
    args: &Args,
    scene: Scene,
    measured: &Measured,
    latencies: Option<&[(Duration, Duration)]>,
) -> Result<()> {
    let (observed, elapsed) = (&measured.observed, measured.elapsed);
    let output = &args.output;
    let mut frames = BufWriter::new(File::create(output.join("frames.csv"))?);
    writeln!(frames, "ust_us,msc")?;
    for (ust, msc) in &observed.frames {
        writeln!(frames, "{ust},{msc}")?;
    }
    frames.flush()?;
    let mut processes = BufWriter::new(File::create(output.join("processes.csv"))?);
    writeln!(
        processes,
        "phase,process,seconds,cpu_ticks,clock_ticks_per_second,cpu_percent_one_core,rss_before_kib,rss_after_kib"
    )?;
    let deltas: Vec<Delta> =
        measured
            .before
            .write_delta(&measured.after, scene.name(), elapsed, &mut processes)?;
    processes.flush()?;

    // Intervals only between consecutive completions; a skipped vblank advances MSC by more.
    let mut intervals = Vec::new();
    for pair in observed.frames.windows(2) {
        let [(ust0, _), (ust1, _)] = pair else {
            continue;
        };
        ensure!(ust1 >= ust0, "Present timestamps moved backwards");
        intervals.push(f64::from(u32::try_from(ust1 - ust0)?) / 1000.0);
    }
    intervals.sort_by(f64::total_cmp);
    let (skipped, discontinuities) = skipped_vblanks(&observed.frames);
    let (mut open, mut close) = (Vec::new(), Vec::new());
    if let Some(latencies) = latencies {
        let mut file = BufWriter::new(File::create(output.join("latency.csv"))?);
        writeln!(file, "cycle,open_ms,close_ms")?;
        for (cycle, (opened, closed)) in latencies.iter().enumerate() {
            let (opened, closed) = (opened.as_secs_f64() * 1e3, closed.as_secs_f64() * 1e3);
            writeln!(file, "{cycle},{opened:.3},{closed:.3}")?;
            open.push(opened);
            close.push(closed);
        }
        file.flush()?;
        open.sort_by(f64::total_cmp);
        close.sort_by(f64::total_cmp);
    }
    let usage = |label: &str| deltas.iter().find(|delta| delta.label == label);
    let cpu = |label: &str| usage(label).map_or_else(String::new, |d| format!("{:.2}", d.cpu));
    let compositor = usage("compositor").context("missing compositor process")?;
    let mut summary = BufWriter::new(File::create(output.join("summary.csv"))?);
    writeln!(
        summary,
        "scene,seconds,completions,damage_events,interval_median_ms,interval_p95_ms,interval_max_ms,skipped_vblanks,msc_discontinuities,compositor_cpu,server_cpu,wm_cpu,probe_cpu,compositor_rss_before_kib,compositor_rss_after_kib,open_median_ms,open_p95_ms,open_max_ms,close_median_ms,close_p95_ms,close_max_ms,gpu_busy_mean,gpu_busy_max"
    )?;
    let gpu = observed.gpu.as_ref().map(|gpu| &gpu.samples);
    let gpu_mean = gpu
        .filter(|samples| !samples.is_empty())
        .map_or_else(String::new, |samples| {
            let total: u32 = samples.iter().copied().map(u32::from).sum();
            format!(
                "{:.1}",
                f64::from(total) / f64::from(u32::try_from(samples.len()).unwrap_or(u32::MAX))
            )
        });
    let gpu_max = gpu
        .and_then(|samples| samples.iter().max())
        .map_or_else(String::new, ToString::to_string);
    writeln!(
        summary,
        "{},{:.3},{},{},{},{},{},{skipped},{discontinuities},{},{},{},{},{},{},{},{},{},{},{},{},{gpu_mean},{gpu_max}",
        scene.name(),
        elapsed.as_secs_f64(),
        observed.frames.len(),
        observed.damage,
        milliseconds(percentile(&intervals, 50)),
        milliseconds(percentile(&intervals, 95)),
        milliseconds(intervals.last().copied()),
        cpu("compositor"),
        cpu("server"),
        cpu("wm"),
        cpu("probe"),
        compositor.rss_before,
        compositor.rss_after,
        milliseconds(percentile(&open, 50)),
        milliseconds(percentile(&open, 95)),
        milliseconds(open.last().copied()),
        milliseconds(percentile(&close, 50)),
        milliseconds(percentile(&close, 95)),
        milliseconds(close.last().copied()),
    )?;
    summary.flush()?;
    Ok(())
}
