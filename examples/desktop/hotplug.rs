use super::{Args, Surface, measure, metrics::Process};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs::create_dir,
    io::{BufRead, Write},
    path::Path,
};
use topology::{Area, Point};
use x11rb::protocol::{
    randr::ConnectionExt as _,
    xproto::{ConfigureWindowAux, ConnectionExt as _, StackMode},
};

#[path = "topology.rs"]
mod topology;

const RED: u32 = 0x00ff_0000;
const GREEN: u32 = 0x0000_ff00;
const BLUE: u32 = 0x0000_00ff;
const YELLOW: u32 = 0x00ff_ff00;
/// Marker size; each check moves it to a new origin.
const MARKER: Area = Area {
    x: 0,
    y: 0,
    width: 128,
    height: 96,
};

/// Keep one override-redirect marker and one managed window alive while an operator changes
/// monitors.
pub(super) fn run(surface: &Surface, args: &Args, processes: &[Process]) -> Result<()> {
    let version = surface.conn.randr_query_version(1, 5)?.reply()?;
    ensure!(
        (version.major_version, version.minor_version) >= (1, 5),
        "monitor sampling requires RandR 1.5"
    );
    // The window manager places and resizes this window like any application's.
    let ordinary = surface.window(BLUE, false)?;
    surface.until("window manager shows the ordinary window", || {
        surface.nudge()?;
        Ok(surface.property(ordinary, "WM_STATE")? == Some(1)
            && surface.window_has_color(ordinary, BLUE)?)
    })?;
    surface.settle()?;
    let marker = surface.window(RED, true)?;
    println!("READY: enter 'sample LABEL' after each monitor transition, then 'quit'");
    std::io::stdout().flush()?;
    let mut count = 0_u32;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let Some(label) = command(&line)? else {
            return Ok(());
        };
        count = count.checked_add(1).context("too many samples")?;
        let output = args.output.join(format!("{count:03}-{label}"));
        create_dir(&output)?;
        let monitors = sample(surface, args, processes, (marker, ordinary), &output)
            .with_context(|| format!("sample {count} ({label})"))?;
        println!("PASS sample={count} label={label} monitors={monitors}");
        std::io::stdout().flush()?;
    }
    bail!("standard input closed before 'quit'")
}

fn command(line: &str) -> Result<Option<&str>> {
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["quit"] => Ok(None),
        ["sample", label]
            if label.len() <= 40
                && label
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') =>
        {
            Ok(Some(label))
        }
        _ => bail!("expected 'sample LABEL' (a-z, 0-9, and '-') or 'quit'"),
    }
}

/// Record the topology, redraw the marker on every monitor and seam and the managed window
/// in place, then measure.
fn sample(
    surface: &Surface,
    args: &Args,
    processes: &[Process],
    (marker, ordinary): (u32, u32),
    output: &Path,
) -> Result<usize> {
    let monitors = topology::capture(surface, output)?;
    ensure!(!monitors.is_empty(), "no active monitor for the marker");
    for monitor in &monitors {
        ensure!(
            monitor.width >= MARKER.width + 40 && monitor.height >= MARKER.height + 80,
            "monitor is too small for the marker"
        );
        // Opposite corners expose buffers that did not follow a size or position change.
        for origin in [
            (monitor.x + 20, monitor.y + 40),
            (
                monitor.x + monitor.width - 20 - MARKER.width,
                monitor.y + monitor.height - 40 - MARKER.height,
            ),
        ] {
            let center = (origin.0 + MARKER.width / 2, origin.1 + MARKER.height / 2);
            flash(surface, marker, origin, &[center])?;
        }
        println!(
            "monitor={}x{}+{}+{} marker_pixels=PASS",
            monitor.width, monitor.height, monitor.x, monitor.y
        );
    }
    for seam in topology::seams(&monitors, &MARKER) {
        flash(surface, marker, seam.origin, &seam.points)?;
        let [(x1, y1), (x2, y2)] = seam.points;
        println!("seam={x1},{y1};{x2},{y2} marker_pixels=PASS");
    }
    redraw(surface, marker, ordinary, &monitors)?;
    println!("ordinary_window_pixels=PASS");
    measure(surface, marker, args, processes, output)?;
    surface.paint(marker, GREEN)?;
    surface.until("marker redraw after measurement", || {
        surface.window_has_color(marker, GREEN)
    })?;
    topology::resources(surface, output)?;
    Ok(monitors.len())
}

/// Repaint the managed window without moving or resizing it, and require each color on
/// screen. A compositor still showing a pixmap from before a resize keeps the old color.
fn redraw(surface: &Surface, marker: u32, ordinary: u32, monitors: &[Area]) -> Result<()> {
    let center = surface.center(ordinary)?;
    let center = (i32::from(center.0), i32::from(center.1));
    // Park the marker in a monitor corner that leaves the window's center visible.
    let origin = monitors
        .iter()
        .flat_map(|monitor| {
            [
                (monitor.x + 20, monitor.y + 40),
                (
                    monitor.x + monitor.width - 20 - MARKER.width,
                    monitor.y + monitor.height - 40 - MARKER.height,
                ),
            ]
        })
        .find(|origin| {
            !(origin.0..origin.0 + MARKER.width).contains(&center.0)
                || !(origin.1..origin.1 + MARKER.height).contains(&center.1)
        })
        .context("no monitor corner leaves the ordinary window visible")?;
    place(surface, marker, origin)?;
    for color in [YELLOW, BLUE] {
        surface.paint(ordinary, color)?;
        surface
            .until("ordinary window pixels", || {
                surface.window_has_color(ordinary, color)
            })
            .context(
                "the managed window did not show its new contents; it is stale, hidden, or covered",
            )?;
    }
    Ok(())
}

fn place(surface: &Surface, marker: u32, origin: Point) -> Result<()> {
    surface
        .conn
        .configure_window(
            marker,
            &ConfigureWindowAux::new()
                .x(origin.0)
                .y(origin.1)
                .width(u32::try_from(MARKER.width)?)
                .height(u32::try_from(MARKER.height)?)
                .stack_mode(StackMode::ABOVE),
        )?
        .check()?;
    Ok(())
}

/// Move the marker, then require each color at every root point in turn.
fn flash(surface: &Surface, marker: u32, origin: Point, points: &[Point]) -> Result<()> {
    place(surface, marker, origin)?;
    let points = points
        .iter()
        .map(|&(x, y)| Ok((i16::try_from(x)?, i16::try_from(y)?)))
        .collect::<Result<Vec<_>>>()?;
    for color in [GREEN, RED] {
        surface.paint(marker, color)?;
        surface.until("monitor marker pixels", || {
            for &point in &points {
                if surface.pixel(point)? != color {
                    return Ok(false);
                }
            }
            Ok(true)
        })?;
    }
    Ok(())
}
