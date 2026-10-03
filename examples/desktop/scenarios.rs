use super::Surface;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use x11rb::protocol::xproto::ConnectionExt as _;

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
                && surface.pixel((80, 80))? == 0x0018_1820,
        )
    })?;
    surface.screenshot(&output.join("empty-workspace.ppm"))?;
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
