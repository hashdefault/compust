use super::Surface;
use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use std::path::Path;
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt as _, CreateGCAux, PropMode, Rectangle},
    wrapper::ConnectionExt as _,
};

/// Color Compust paints where no window or wallpaper covers the root.
pub(super) const BACKGROUND: u32 = 0x0018_1820;

const RED: u32 = 0x00ff_0000;
const BLUE: u32 = 0x0000_00ff;
const GREEN: u32 = 0x0000_ff00;

/// How the window manager under test arranges ordinary windows.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Layout {
    /// Clients are resized to fill the screen and never overlap, as in Xmonad.
    Tiling,
    /// Clients keep their size inside decorated frames and may overlap, as in Openbox.
    Stacking,
}

/// Which scenarios a window manager supports.
#[derive(Clone, Copy)]
pub(super) struct Plan {
    pub(super) layout: Layout,
    /// Clients sit in reparenting frames with a painted title bar.
    pub(super) frames: bool,
    /// The second workspace exists only while a window is assigned to it.
    pub(super) anchor: bool,
}

pub(super) fn exercise(surface: &Surface, output: &Path, plan: Plan) -> Result<u32> {
    let layout = plan.layout;
    let anchor = if plan.anchor {
        let anchor = surface.anchor()?;
        surface.until("second workspace for the anchor window", || {
            Ok(surface.property(surface.root, "_NET_NUMBER_OF_DESKTOPS")? >= Some(2))
        })?;
        Some(anchor)
    } else {
        None
    };
    let red = surface.window(RED, false)?;
    let blue = surface.window(BLUE, false)?;
    surface.until("window manager maps both clients", || {
        if layout == Layout::Stacking {
            // Openbox 3.6.1 can leave map requests unread when they arrive while it reacts
            // to the compositor starting; it handles them with the next event it receives.
            surface.nudge()?;
        }
        Ok(surface.property(red, "WM_STATE")? == Some(1)
            && surface.property(blue, "WM_STATE")? == Some(1))
    })?;
    if layout == Layout::Stacking {
        surface.settle()?;
        surface.place(red, (60, 80))?;
        surface.place(blue, (460, 80))?;
        surface.until("stacking clients placed apart", || {
            Ok(surface.origin(red)?.0 + 320 <= surface.origin(blue)?.0)
        })?;
    }
    if plan.frames {
        for (window, color) in [(red, RED), (blue, BLUE)] {
            framed(surface, window, color, layout)?;
        }
    }
    surface.until("managed client pixels", || {
        Ok(surface.window_has_color(red, RED)? && surface.window_has_color(blue, BLUE)?)
    })?;
    surface.screenshot(&output.join("managed.ppm"))?;
    let original = surface.conn.get_geometry(red)?.reply()?;
    ensure!(
        layout == Layout::Stacking || original.width != 320 || original.height != 240,
        "window manager did not arrange the client"
    );

    let below = surface.pixel((80, 80))?;
    let popup = surface.window(GREEN, true)?;
    surface.until("popup pixels", || Ok(surface.pixel((80, 80))? == GREEN))?;
    surface.screenshot(&output.join("popup.ppm"))?;
    surface.conn.destroy_window(popup)?.check()?;
    surface.until("popup removal", || Ok(surface.pixel((80, 80))? == below))?;

    if layout == Layout::Stacking {
        restack(surface, red, blue)?;
        iconify(surface, blue)?;
        activate(surface, red)?;
    }
    fullscreen(surface, red, layout, output)?;
    surface.until("window manager fullscreen restore", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        Ok(size.width == original.width
            && size.height == original.height
            && surface.window_has_color(blue, BLUE)?
            && surface.window_has_color(red, RED)?)
    })?;

    surface.message(surface.root, "_NET_CURRENT_DESKTOP", [1, 0, 0, 0, 0])?;
    surface.until("empty workspace", || {
        Ok(
            surface.property(surface.root, "_NET_CURRENT_DESKTOP")? == Some(1)
                && surface.pixel((80, 80))? == BACKGROUND,
        )
    })?;
    surface.screenshot(&output.join("empty-workspace.ppm"))?;
    wallpaper(surface)?;
    surface.message(surface.root, "_NET_CURRENT_DESKTOP", [0, 0, 0, 0, 0])?;
    surface.until("workspace restoration", || {
        Ok(
            surface.property(surface.root, "_NET_CURRENT_DESKTOP")? == Some(0)
                && surface.window_has_color(red, RED)?
                && surface.window_has_color(blue, BLUE)?,
        )
    })?;

    if let Some(anchor) = anchor {
        surface.conn.destroy_window(anchor)?.check()?;
    }
    for _ in 0..32 {
        let transient = surface.window(GREEN, false)?;
        surface.conn.destroy_window(transient)?.check()?;
    }
    let vacated = surface.center(blue)?;
    surface.conn.destroy_window(blue)?.check()?;
    surface.until("survivor after rapid lifecycle", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        let alone = match layout {
            Layout::Tiling => size.width > original.width,
            Layout::Stacking => surface.pixel(vacated)? == BACKGROUND,
        };
        Ok(alone && surface.window_has_color(red, RED)?)
    })?;
    surface.screenshot(&output.join("survivor.ppm"))?;
    Ok(red)
}

/// Require a reparenting frame around `window` with a painted title bar above it.
fn framed(surface: &Surface, window: u32, color: u32, layout: Layout) -> Result<()> {
    let size = surface.conn.get_geometry(window)?.reply()?;
    ensure!(
        layout == Layout::Tiling || (size.width == 320 && size.height == 240),
        "stacking window manager resized the client"
    );
    let frame = surface.frame(window)?;
    ensure!(
        frame != window,
        "window manager did not reparent the client"
    );
    let top = surface.conn.get_geometry(frame)?.reply()?.y;
    let origin = surface.origin(window)?;
    let title = origin.1 - top;
    ensure!(title > 2, "frame has no title bar above the client");
    let middle = origin.0 + i16::try_from(size.width / 2)?;
    surface.until("frame decoration pixels", || {
        let pixel = surface.pixel((middle, top + title / 2))?;
        Ok(pixel != BACKGROUND && pixel != color)
    })?;
    println!("frame={frame:#x} client={window:#x} title_height={title}");
    Ok(())
}

/// Overlap the clients and require each activation to bring its window to the front.
fn restack(surface: &Surface, red: u32, blue: u32) -> Result<()> {
    let home = surface.origin(blue)?;
    surface.place(blue, (220, 180))?;
    surface.until("overlapping clients", || {
        let (lower, upper) = (surface.origin(red)?, surface.origin(blue)?);
        Ok(upper != home
            && (lower.0..lower.0 + 300).contains(&upper.0)
            && (lower.1..lower.1 + 220).contains(&upper.1))
    })?;
    let upper = surface.origin(blue)?;
    let shared = (upper.0 + 10, upper.1 + 10);
    for (window, color, description) in [
        (blue, BLUE, "blue raised over red"),
        (red, RED, "red raised over blue"),
        (blue, BLUE, "blue raised again"),
    ] {
        activate(surface, window)?;
        surface.until(description, || Ok(surface.pixel(shared)? == color))?;
    }
    surface.place(blue, (460, 80))?;
    surface.until("clients apart again", || {
        Ok(surface.pixel(shared)? == RED && surface.window_has_color(blue, BLUE)?)
    })
}

/// Iconify a client through the window manager, then map it again.
fn iconify(surface: &Surface, window: u32) -> Result<()> {
    let center = surface.center(window)?;
    surface.message(window, "WM_CHANGE_STATE", [3, 0, 0, 0, 0])?;
    surface.until("iconified window leaves the screen", || {
        Ok(
            surface.property(window, "WM_STATE")? == Some(3)
                && surface.pixel(center)? == BACKGROUND,
        )
    })?;
    surface.conn.map_window(window)?.check()?;
    surface.until("iconified window returns", || {
        Ok(surface.property(window, "WM_STATE")? == Some(1)
            && surface.window_has_color(window, BLUE)?)
    })
}

fn activate(surface: &Surface, window: u32) -> Result<()> {
    // Source indication 2: a pager, which window managers do not treat as focus stealing.
    surface.message(window, "_NET_ACTIVE_WINDOW", [2, 0, 0, 0, 0])
}

/// Enter EWMH fullscreen, check the client covers the screen, and request restoration.
fn fullscreen(surface: &Surface, red: u32, layout: Layout, output: &Path) -> Result<()> {
    let fullscreen = surface.atom("_NET_WM_STATE_FULLSCREEN")?;
    surface.message(red, "_NET_WM_STATE", [1, fullscreen, 0, 1, 0])?;
    let fullscreen_result = surface.until("window manager fullscreen", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        let at_origin = match layout {
            Layout::Tiling => size.x == 0 && size.y == 0,
            Layout::Stacking => surface.origin(red)? == (0, 0),
        };
        Ok(
            u32::from(size.width) + 2 * u32::from(size.border_width) == u32::from(surface.width)
                && u32::from(size.height) + 2 * u32::from(size.border_width)
                    == u32::from(surface.height)
                && at_origin
                && surface
                    .properties(red, "_NET_WM_STATE")?
                    .contains(&fullscreen)
                && surface.pixel((80, 80))? == RED,
        )
    });
    fullscreen_result.with_context(|| {
        format!(
            "fullscreen geometry: {:?}; pixel: {:?}",
            surface
                .conn
                .get_geometry(red)
                .map(|cookie| cookie.reply().map(|size| (
                    size.width,
                    size.height,
                    size.border_width,
                    size.x,
                    size.y
                ))),
            surface.pixel((80, 80))
        )
    })?;
    let fullscreen_geometry = surface.conn.get_geometry(red)?.reply()?;
    println!(
        "fullscreen client={}x{} border={}",
        fullscreen_geometry.width, fullscreen_geometry.height, fullscreen_geometry.border_width
    );
    surface.screenshot(&output.join("fullscreen.ppm"))?;
    surface.message(red, "_NET_WM_STATE", [0, fullscreen, 0, 1, 0])
}

/// Show, then remove, a tiled root wallpaper on the empty workspace.
fn wallpaper(surface: &Surface) -> Result<()> {
    let color = 0x0033_6699;
    let pixmap = surface.conn.generate_id()?;
    surface
        .conn
        .create_pixmap(24, pixmap, surface.root, 4, 4)?
        .check()?;
    let gc = surface.conn.generate_id()?;
    surface
        .conn
        .create_gc(gc, pixmap, &CreateGCAux::new().foreground(color))?
        .check()?;
    let area = Rectangle {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    };
    surface
        .conn
        .poly_fill_rectangle(pixmap, gc, &[area])?
        .check()?;
    surface.conn.free_gc(gc)?.check()?;
    let property = surface.atom("_XROOTPMAP_ID")?;
    surface
        .conn
        .change_property32(
            PropMode::REPLACE,
            surface.root,
            property,
            AtomEnum::PIXMAP,
            &[pixmap],
        )?
        .check()?;
    surface.until("wallpaper pixels", || Ok(surface.pixel((80, 80))? == color))?;
    surface
        .conn
        .delete_property(surface.root, property)?
        .check()?;
    surface.until("wallpaper removal", || {
        Ok(surface.pixel((80, 80))? == BACKGROUND)
    })?;
    surface.conn.free_pixmap(pixmap)?.check()?;
    Ok(())
}

/// Make the survivor translucent; with blur configured, every redraw blurs what lies behind it.
pub(super) fn translucent(surface: &Surface, window: u32, opacity: u8) -> Result<()> {
    let value = u32::try_from(u64::from(u32::MAX) * u64::from(opacity) / 100)?;
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
    surface.until("translucent survivor pixels", || {
        shows_red(surface, window, opacity)
    })
}

/// Whether `window` shows red at `opacity` percent over the background. Translucent pixels
/// allow for rounding in the compositor's 8-bit alpha mask.
pub(super) fn shows_red(surface: &Surface, window: u32, opacity: u8) -> Result<bool> {
    let pixel = surface.window_pixel(window)?;
    let expected = blend(0x00ff_0000, opacity);
    Ok(if opacity == 100 {
        pixel == expected
    } else {
        near(pixel, expected)
    })
}

/// An opaque `color` composited at `opacity` percent over the background.
fn blend(color: u32, opacity: u8) -> u32 {
    let alpha = u32::from(opacity);
    [16, 8, 0].into_iter().fold(0, |pixel, shift| {
        let source = (color >> shift) & 255;
        let below = (BACKGROUND >> shift) & 255;
        pixel | ((source * alpha + below * (100 - alpha) + 50) / 100) << shift
    })
}

fn near(pixel: u32, expected: u32) -> bool {
    [16, 8, 0]
        .into_iter()
        .all(|shift| ((pixel >> shift) & 255).abs_diff((expected >> shift) & 255) <= 3)
}
