use crate::support::resources::ResourceProgress;
use crate::{rect, regions::area, support::Desktop};
use anyhow::{Context, Result, ensure};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::*,
    },
};

const PLAIN: &str = "fade_ms = 0\nblur_radius = 0\nmax_fps = 1000\nvsync = false";
const EFFECTS: &str = "fade_ms = 30\nblur_radius = 4\nshadow_radius = 8\ncorner_radius = 6\nmax_fps = 1000\nvsync = false";
const BACKGROUND: [u8; 3] = [24, 24, 32];

#[test]
#[ignore = "10,000 real X11 window cycles; set COMPUST_ENDURANCE_CYCLES to override"]
fn endurance_cycles_keep_resources_stable() -> Result<()> {
    let cycles = match std::env::var("COMPUST_ENDURANCE_CYCLES") {
        Ok(value) => value.parse().context("invalid COMPUST_ENDURANCE_CYCLES")?,
        Err(std::env::VarError::NotPresent) => 10_000,
        Err(error) => return Err(error.into()),
    };
    endurance(cycles)
}

#[test]
fn short_endurance_cycles_keep_resources_stable() -> Result<()> {
    endurance(40)
}

fn endurance(cycles: usize) -> Result<()> {
    ensure!(cycles > 0, "endurance needs at least one cycle");
    let mut desktop = Desktop::new(PLAIN)?;
    let neighbor = desktop.window(rect(180, 20), 0x0000_00ff)?;
    // A browser-like client leaves new storage unpainted until its own redraw.
    desktop
        .conn
        .change_window_attributes(
            neighbor,
            &ChangeWindowAttributesAux::new().background_pixmap(x11rb::NONE),
        )?
        .check()?;
    desktop.map(neighbor)?;
    desktop.fill(neighbor, area(0, 0, 100, 100), 0x0000_00ff)?;
    desktop.until_pixel((220, 60), |p| p == [0, 0, 255])?;
    let marker = desktop.window(area(250, 180, 40, 40), 0x00ff_0000)?;
    desktop.map(marker)?;
    desktop.until_pixel((250, 180), |p| p == [255, 0, 0])?;
    for cycle in 0..100.min(cycles / 10) {
        Cycle {
            index: cycle,
            effects: false,
        }
        .run(&desktop, neighbor)
        .context("endurance warmup")?;
    }
    let baseline = desktop.resources()?;
    let mut progress = ResourceProgress::new("endurance")?;
    progress.checkpoint(&desktop, 0)?;
    let reload_every = 250.min((cycles / 2).max(1));
    let mut effects = false;
    for cycle in 0..cycles {
        if cycle > 0 && cycle.is_multiple_of(reload_every) {
            effects = !effects;
            reload(&desktop, effects)?;
        }
        Cycle {
            index: cycle,
            effects,
        }
        .run(&desktop, neighbor)
        .with_context(|| format!("endurance cycle {}, effects={effects}", cycle + 1))?;
        if (cycle + 1).is_multiple_of(1000) && cycle + 1 < cycles {
            progress.checkpoint(&desktop, cycle + 1)?;
        }
    }
    if effects {
        reload(&desktop, false)?;
    }
    let final_resources = desktop.until_resources(&baseline)?;
    progress.checkpoint(&desktop, cycles)?;
    assert_eq!(final_resources, baseline, "endurance after {cycles} cycles");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

fn reload(desktop: &Desktop, effects: bool) -> Result<()> {
    desktop.reload(if effects { EFFECTS } else { PLAIN })?;
    // The marker's corner changes only when the new corner radius is applied.
    desktop.until_pixel((250, 180), |p| (p != [255, 0, 0]) == effects)?;
    desktop.until_pixel((270, 200), |p| p == [255, 0, 0])?;
    Ok(())
}

struct Cycle {
    index: usize,
    effects: bool,
}

impl Cycle {
    fn run(&self, desktop: &Desktop, neighbor: Window) -> Result<()> {
        let cycle = self.index;
        let kind = cycle % 7;
        let window = match kind {
            1 | 5 => desktop.argb_window(rect(20, 20))?,
            _ => desktop.window(rect(20, 20), 0x00ff_0000)?,
        };
        let colormap = if matches!(kind, 1 | 5) {
            Some(
                desktop
                    .conn
                    .get_window_attributes(window)?
                    .reply()?
                    .colormap,
            )
        } else {
            None
        };
        let child = if kind == 5 {
            Some(browser_child(desktop, window)?)
        } else {
            None
        };
        properties(desktop, window, kind)?;
        desktop.map(window)?;
        let drawable = child.unwrap_or(window);
        let color = if kind == 1 { 0x8080_0000 } else { 0xffff_0000 };
        desktop.fill(drawable, area(0, 0, 100, 100), color)?;
        let red = |p: [u8; 3]| {
            if matches!(kind, 1 | 2) {
                matches!(p, [136..=142, 11..=13, 15..=17])
            } else {
                p == [255, 0, 0]
            }
        };
        desktop.until_pixel((60, 60), red)?;
        desktop
            .conn
            .configure_window(window, &ConfigureWindowAux::new().width(140))?
            .check()?;
        if let Some(child) = child {
            desktop
                .conn
                .configure_window(child, &ConfigureWindowAux::new().width(140))?
                .check()?;
        }
        desktop.fill(drawable, area(0, 0, 140, 100), color)?;
        desktop.until_pixel((140, 60), red)?;
        if kind == 3 {
            desktop.until_pixel((60, 110), |p| p == BACKGROUND)?;
        }
        // Keep close and the tiling manager's resize in one server transaction.
        desktop.conn.grab_server()?.check()?;
        match cycle % 3 {
            0 | 1 => desktop.conn.unmap_window(window)?.check()?,
            _ => desktop.conn.destroy_window(window)?.check()?,
        }
        if cycle % 3 == 1 {
            desktop.conn.destroy_window(window)?.check()?;
        }
        if kind == 6 && self.effects {
            desktop
                .conn
                .configure_window(neighbor, &ConfigureWindowAux::new().x(20).width(260))?
                .check()?;
        }
        desktop.conn.ungrab_server()?.check()?;
        desktop.until_pixel((60, 60), |p| p == BACKGROUND)?;
        desktop.until_pixel((140, 60), |p| p == BACKGROUND)?;
        if kind == 6 {
            if !self.effects {
                desktop
                    .conn
                    .configure_window(neighbor, &ConfigureWindowAux::new().x(20).width(260))?
                    .check()?;
            }
            // The previous capture remains in place until the client redraws the whole window.
            desktop.fill(neighbor, area(0, 0, 260, 100), 0x0000_00ff)?;
            desktop.until_pixel((60, 60), |p| p == [0, 0, 255])?;
            desktop
                .conn
                .configure_window(neighbor, &ConfigureWindowAux::new().x(180).width(100))?
                .check()?;
            desktop.fill(neighbor, area(0, 0, 100, 100), 0x0000_00ff)?;
            desktop.until_pixel((60, 60), |p| p == BACKGROUND)?;
            desktop.until_pixel((220, 60), |p| p == [0, 0, 255])?;
        }
        // In the unmap-only case, reclaim the client's window after observing its full close.
        if cycle.is_multiple_of(3) {
            desktop.conn.destroy_window(window)?.check()?;
        }
        if let Some(colormap) = colormap {
            desktop.conn.free_colormap(colormap)?.check()?;
        }
        Ok(())
    }
}

fn properties(desktop: &Desktop, window: Window, kind: usize) -> Result<()> {
    match kind {
        2 => desktop.opacity(window, 0x8000_0000)?,
        3 => desktop
            .conn
            .shape_rectangles(
                SO::SET,
                SK::BOUNDING,
                ClipOrdering::UNSORTED,
                window,
                0,
                0,
                &[area(0, 0, 140, 70)],
            )?
            .check()?,
        4 => desktop
            .conn
            .change_window_attributes(
                window,
                &ChangeWindowAttributesAux::new().override_redirect(1),
            )?
            .check()?,
        _ => {}
    }
    Ok(())
}

fn browser_child(desktop: &Desktop, parent: Window) -> Result<Window> {
    let child = desktop.conn.generate_id()?;
    let screen = desktop.conn.setup().roots.first().context("no X screen")?;
    desktop
        .conn
        .create_window(
            24,
            child,
            parent,
            0,
            0,
            100,
            100,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &CreateWindowAux::new()
                .background_pixmap(x11rb::NONE)
                .border_pixel(0)
                .colormap(screen.default_colormap),
        )?
        .check()?;
    desktop.map(child)?;
    Ok(child)
}
