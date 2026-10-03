#[path = "desktop/metrics.rs"]
mod metrics;
#[path = "desktop/scenarios.rs"]
mod scenarios;
#[path = "desktop/surface.rs"]
mod surface;

use anyhow::{Context, Result, ensure};
use clap::Parser;
use metrics::{Process, Snapshot};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
use surface::Surface;
use x11rb::{
    connection::Connection,
    protocol::{Event, present::CompleteKind},
};

#[derive(Parser)]
#[command(about = "Exercise an isolated EWMH desktop and record a release baseline")]
struct Args {
    /// Isolated test display. This probe creates windows and switches workspaces.
    #[arg(long)]
    display: String,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=30), default_value_t = 10)]
    seconds: u32,
    /// Linux process to measure, formatted as label:pid.
    #[arg(long, required = true)]
    process: Vec<Process>,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let surface = Surface::connect(&args.display)?;
    println!(
        "vendor={} release={} geometry={}x{} depth=24",
        String::from_utf8_lossy(&surface.conn.setup().vendor),
        surface.conn.setup().release_number,
        surface.width,
        surface.height,
    );
    let window = scenarios::exercise(&surface, &args.output)?;
    surface.subscribe()?;
    let mut frames = BufWriter::new(File::create(args.output.join("frames.csv"))?);
    writeln!(frames, "phase,serial,ust_us,msc,mode")?;
    let mut cpu = BufWriter::new(File::create(args.output.join("processes.csv"))?);
    writeln!(
        cpu,
        "phase,process,seconds,cpu_ticks,clock_ticks_per_second,cpu_percent_one_core,rss_before_kib,rss_after_kib"
    )?;
    let mut processes = args.process;
    processes.push(Process::probe());
    run_phase(&surface, window, Duration::from_secs(2), |_| Ok(()))?;
    for (phase, active) in [("idle", false), ("active", true)] {
        while let Some(event) = surface.conn.poll_for_event()? {
            if let Event::Error(error) = event {
                anyhow::bail!("probe X11 error between phases: {error:?}");
            }
        }
        let before = Snapshot::read(&processes)?;
        let started = Instant::now();
        let mut count = 0_u32;
        let mut last_ust = None;
        let duration = Duration::from_secs(u64::from(args.seconds));
        let updates = run_phase(
            &surface,
            if active { window } else { 0 },
            duration,
            |event| {
                if let Event::PresentCompleteNotify(event) = event
                    && event.event == surface.present
                    && event.kind == CompleteKind::PIXMAP
                {
                    if let Some(previous) = last_ust {
                        ensure!(event.ust >= previous, "Present timestamps moved backwards");
                    }
                    last_ust = Some(event.ust);
                    count += 1;
                    writeln!(
                        frames,
                        "{phase},{},{},{},{:?}",
                        event.serial, event.ust, event.msc, event.mode
                    )?;
                }
                Ok(())
            },
        )?;
        let elapsed = started.elapsed();
        let after = Snapshot::read(&processes)?;
        before.write_delta(&after, phase, elapsed, &mut cpu)?;
        if active {
            ensure!(count > 1, "no usable compositor Present completion samples");
        }
        println!(
            "phase={phase} seconds={:.6} updates={updates} completions={count}",
            elapsed.as_secs_f64()
        );
    }
    frames.flush()?;
    cpu.flush()?;
    surface.paint(window, 0x00ff_0000)?;
    surface.until("final survivor pixels", || {
        surface.window_has_color(window, 0x00ff_0000)
    })?;
    surface.screenshot(&args.output.join("final.ppm"))?;
    println!(
        "PASS: managed windows, popup removal, fullscreen restore, workspace return, rapid lifecycle, survivor redraw"
    );
    Ok(())
}

fn run_phase(
    surface: &Surface,
    window: u32,
    duration: Duration,
    mut record: impl FnMut(Event) -> Result<()>,
) -> Result<u32> {
    let start = Instant::now();
    let end = start + duration;
    let mut frame = 0_u32;
    loop {
        while let Some(event) = surface.conn.poll_for_event()? {
            if let Event::Error(error) = &event {
                anyhow::bail!("probe X11 error: {error:?}");
            }
            record(event)?;
        }
        let now = Instant::now();
        if now >= end {
            return Ok(frame);
        }
        let next = start + Duration::from_secs_f64(f64::from(frame) / 60.0);
        if window != 0 && now >= next {
            let color = if frame.is_multiple_of(2) {
                0x00ff_0000
            } else {
                0x0000_00ff
            };
            surface.paint(window, color)?;
            frame += 1;
        } else {
            let deadline = if window == 0 { end } else { next.min(end) };
            surface
                .wait(deadline)
                .context("waiting for desktop activity")?;
        }
    }
}
