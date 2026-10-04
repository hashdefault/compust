use super::scene::{BLUE, GREEN, WHITE, activate, create, suspended, typed, wait_state};
use crate::{
    Args, Presentation,
    metrics::{Process, Snapshot},
    run_phase,
    surface::Surface,
};
use anyhow::{Result, ensure};
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Duration, Instant},
};
use x11rb::{
    NONE,
    protocol::{
        Event,
        present::CompleteKind,
        xproto::{ConnectionExt as _, Rectangle},
    },
};

pub(super) fn check(surface: &Surface, args: &Args, processes: &[Process]) -> Result<()> {
    let full = create(
        surface,
        Rectangle {
            x: 0,
            y: 0,
            width: surface.width,
            height: surface.height,
        },
        BLUE,
    )?;
    activate(surface, full)?;
    surface.conn.map_window(full)?.check()?;
    wait_state(surface, true)?;
    ensure!(
        surface.screen_pixel((160, 120))? == BLUE,
        "fullscreen window is not visible directly"
    );
    let popup = create(
        surface,
        Rectangle {
            x: 20,
            y: 20,
            width: 60,
            height: 40,
        },
        GREEN,
    )?;
    typed(surface, popup, "_NET_WM_WINDOW_TYPE_DIALOG")?;
    surface.conn.map_window(popup)?.check()?;
    wait_state(surface, false)?;
    surface.until("popup restores compositing", || {
        Ok(surface.pixel((40, 40))? == GREEN && surface.pixel((160, 120))? == BLUE)
    })?;
    let mut frames = BufWriter::new(File::create(args.output.join("frames.csv"))?);
    writeln!(frames, "phase,serial,ust_us,msc,mode")?;
    let mut cpu = BufWriter::new(File::create(args.output.join("processes.csv"))?);
    writeln!(
        cpu,
        "phase,process,seconds,cpu_ticks,clock_ticks_per_second,cpu_percent_one_core,rss_before_kib,rss_after_kib"
    )?;
    let mut phases = File::create(args.output.join("fullscreen.csv"))?;
    writeln!(
        phases,
        "phase,seconds,updates,present_completions,overlay_damage,suspended"
    )?;
    let scene = Fullscreen {
        surface,
        window: full,
        args,
        processes,
    };
    let mut records = Records {
        frames,
        cpu,
        phases,
    };
    for state in [false, true] {
        if state {
            surface.conn.destroy_window(popup)?.check()?;
            wait_state(surface, true)?;
        }
        scene.measure(state, &mut records)?;
    }
    surface.paint(full, GREEN)?;
    ensure!(
        surface.screen_pixel((160, 120))? == GREEN,
        "unredirected redraw did not reach the root"
    );
    let popup = create(
        surface,
        Rectangle {
            x: 20,
            y: 20,
            width: 60,
            height: 40,
        },
        BLUE,
    )?;
    typed(surface, popup, "_NET_WM_WINDOW_TYPE_DIALOG")?;
    surface.conn.map_window(popup)?.check()?;
    wait_state(surface, false)?;
    surface.until("recapture after suspension", || {
        Ok(surface.pixel((160, 120))? == GREEN && surface.pixel((40, 40))? == BLUE)
    })?;
    surface.screenshot(&args.output.join("fullscreen-resumed.ppm"))?;
    surface.conn.destroy_window(full)?.check()?;
    surface.until("fullscreen removal", || {
        Ok(surface.pixel((160, 120))? == WHITE)
    })?;
    surface.conn.destroy_window(popup)?.check()?;
    activate(surface, NONE)?;
    wait_state(surface, true)?;
    records.frames.flush()?;
    records.cpu.flush()?;
    println!("PASS: fullscreen suspension, popup resume, fresh recapture, cover removal");
    Ok(())
}

struct Records {
    frames: BufWriter<File>,
    cpu: BufWriter<File>,
    phases: File,
}

struct Fullscreen<'a> {
    surface: &'a Surface,
    window: u32,
    args: &'a Args,
    processes: &'a [Process],
}

impl Fullscreen<'_> {
    fn measure(&self, state: bool, records: &mut Records) -> Result<()> {
        let Self {
            surface,
            window,
            args,
            processes,
        } = *self;
        run_phase(surface, window, Duration::from_secs(2), |_| Ok(()))?;
        ensure!(
            suspended(surface)? == state,
            "overlay state changed during warmup"
        );
        while surface.poll_event()?.is_some() {}
        let before = Snapshot::read(processes)?;
        let started = Instant::now();
        let mut completions = 0_u32;
        let mut damage = 0_u32;
        let phase = if state { "suspended" } else { "composed" };
        let updates = run_phase(
            surface,
            window,
            Duration::from_secs(u64::from(args.seconds)),
            |event| {
                match event {
                    Event::PresentCompleteNotify(event)
                        if event.event == surface.present && event.kind == CompleteKind::PIXMAP =>
                    {
                        completions += 1;
                        writeln!(
                            records.frames,
                            "{phase},{},{},{},{:?}",
                            event.serial, event.ust, event.msc, event.mode
                        )?;
                    }
                    Event::DamageNotify(event) if event.drawable == surface.overlay => damage += 1,
                    Event::MapNotify(event) if state && event.window == surface.overlay => {
                        anyhow::bail!("overlay mapped during suspension")
                    }
                    Event::UnmapNotify(event) if !state && event.window == surface.overlay => {
                        anyhow::bail!("overlay unmapped during compositing")
                    }
                    _ => {}
                }
                Ok(())
            },
        )?;
        let elapsed = started.elapsed();
        before.write_delta(
            &Snapshot::read(processes)?,
            phase,
            elapsed,
            &mut records.cpu,
        )?;
        ensure!(
            suspended(surface)? == state,
            "overlay changed state during {phase}"
        );
        if state {
            ensure!(
                completions == 0 && damage == 0,
                "compositor kept rendering while suspended"
            );
        } else {
            match args.presentation {
                Presentation::Present => {
                    ensure!(completions > 1, "no usable Present samples while composed");
                }
                Presentation::Direct => ensure!(
                    completions == 0 && damage > 1,
                    "direct copying lacks redraws or used Present"
                ),
            }
        }
        writeln!(
            records.phases,
            "{phase},{:.6},{updates},{completions},{damage},{state}",
            elapsed.as_secs_f64()
        )?;
        println!("phase={phase} updates={updates} completions={completions} damage={damage}");
        Ok(())
    }
}
