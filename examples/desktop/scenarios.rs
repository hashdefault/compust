use super::Surface;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt as _, CreateGCAux, PropMode, Rectangle},
    wrapper::ConnectionExt as _,
};

/// Color Compust paints where no window or wallpaper covers the root.
pub(super) const BACKGROUND: u32 = 0x0018_1820;

pub(super) fn exercise(surface: &Surface, output: &Path) -> Result<u32> {
    let red = surface.window(0x00ff_0000, false)?;
    let blue = surface.window(0x0000_00ff, false)?;
    surface.until("window manager maps both clients", || {
        Ok(surface.property(red, "WM_STATE")? == Some(1)
            && surface.property(blue, "WM_STATE")? == Some(1)
            && surface.window_has_color(red, 0x00ff_0000)?
            && surface.window_has_color(blue, 0x0000_00ff)?)
    })?;
    surface.screenshot(&output.join("managed.ppm"))?;
    let original = surface.conn.get_geometry(red)?.reply()?;
    ensure!(
        original.width != 320 || original.height != 240,
        "window manager did not arrange the client"
    );

    let below = surface.pixel((80, 80))?;
    let popup = surface.window(0x0000_ff00, true)?;
    surface.until("popup pixels", || {
        Ok(surface.pixel((80, 80))? == 0x0000_ff00)
    })?;
    surface.screenshot(&output.join("popup.ppm"))?;
    surface.conn.destroy_window(popup)?.check()?;
    surface.until("popup removal", || Ok(surface.pixel((80, 80))? == below))?;

    let fullscreen = surface.atom("_NET_WM_STATE_FULLSCREEN")?;
    surface.message(red, "_NET_WM_STATE", [1, fullscreen, 0, 1, 0])?;
    let fullscreen_result = surface.until("window manager fullscreen", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        Ok(
            u32::from(size.width) + 2 * u32::from(size.border_width) == u32::from(surface.width)
                && u32::from(size.height) + 2 * u32::from(size.border_width)
                    == u32::from(surface.height)
                && size.x == 0
                && size.y == 0
                && surface.property(red, "_NET_WM_STATE")? == Some(fullscreen)
                && surface.pixel((80, 80))? == 0x00ff_0000,
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
    surface.message(red, "_NET_WM_STATE", [0, fullscreen, 0, 1, 0])?;
    surface.until("window manager fullscreen restore", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        Ok(size.width == original.width
            && size.height == original.height
            && surface.window_has_color(blue, 0x0000_00ff)?
            && surface.window_has_color(red, 0x00ff_0000)?)
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
                && surface.window_has_color(red, 0x00ff_0000)?
                && surface.window_has_color(blue, 0x0000_00ff)?,
        )
    })?;

    for _ in 0..32 {
        let transient = surface.window(0x0000_ff00, false)?;
        surface.conn.destroy_window(transient)?.check()?;
    }
    surface.conn.destroy_window(blue)?.check()?;
    surface.until("survivor after rapid lifecycle", || {
        let size = surface.conn.get_geometry(red)?.reply()?;
        Ok(size.width > original.width && surface.window_has_color(red, 0x00ff_0000)?)
    })?;
    surface.screenshot(&output.join("survivor.ppm"))?;
    Ok(red)
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
