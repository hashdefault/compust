use crate::{rect, support::Desktop};
use anyhow::{Context, Result};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK},
        xproto::*,
    },
    wrapper::ConnectionExt as _,
};

#[test]
fn repaints_damage_and_live_opacity_changes() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    let gc = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_gc(gc, window, &CreateGCAux::new().foreground(0x0000_ff00))?
        .check()?;
    desktop
        .conn
        .poly_fill_rectangle(window, gc, &[rect(0, 0)])?
        .check()?;
    desktop.until_pixel((40, 40), |p| p == [0, 255, 0])?;
    desktop.opacity(window, 0)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    desktop.opacity(window, u32::MAX)?;
    desktop.until_pixel((40, 40), |p| p == [0, 255, 0])?;
    Ok(())
}

#[test]
fn composites_premultiplied_argb_visuals() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let background = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(background)?;
    let visual = desktop
        .conn
        .setup()
        .roots
        .first()
        .context("screen")?
        .allowed_depths
        .iter()
        .find(|d| d.depth == 32)
        .and_then(|d| d.visuals.first())
        .context("ARGB visual")?
        .visual_id;
    let colormap = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_colormap(ColormapAlloc::NONE, colormap, desktop.root, visual)?
        .check()?;
    let window = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            32,
            window,
            desktop.root,
            20,
            20,
            100,
            100,
            0,
            WindowClass::INPUT_OUTPUT,
            visual,
            &CreateWindowAux::new()
                .background_pixel(0x8000_8000)
                .border_pixel(0)
                .colormap(colormap),
        )?
        .check()?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |[r, g, b]| {
        (126..=129).contains(&r) && (126..=129).contains(&g) && b == 0
    })?;
    Ok(())
}

#[test]
fn follows_client_opacity_inside_window_manager_frame() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            24,
            client,
            frame,
            10,
            10,
            80,
            80,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().background_pixel(0x0000_00ff),
        )?
        .check()?;
    let state = desktop.conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, client, state, state, &[1, 0])?
        .check()?;
    desktop.map(client)?;
    desktop.map(frame)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.opacity(client, 0)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn tracks_stacking_and_wallpaper_without_intercepting_input() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let upper = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(upper)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop
        .conn
        .configure_window(
            upper,
            &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
        )?
        .check()?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    let pixmap = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_pixmap(24, pixmap, desktop.root, 1, 1)?
        .check()?;
    let gc = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_gc(gc, pixmap, &CreateGCAux::new().foreground(0x0000_ff00))?
        .check()?;
    desktop
        .conn
        .poly_fill_rectangle(
            pixmap,
            gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }],
        )?
        .check()?;
    let atom = desktop
        .conn
        .intern_atom(false, b"_XROOTPMAP_ID")?
        .reply()?
        .atom;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            desktop.root,
            atom,
            AtomEnum::PIXMAP,
            &[pixmap],
        )?
        .check()?;
    desktop.until_pixel((200, 200), |p| p == [0, 255, 0])?;
    assert!(
        desktop
            .conn
            .shape_get_rectangles(desktop.overlay, SK::INPUT)?
            .reply()?
            .rectangles
            .is_empty()
    );
    Ok(())
}

#[test]
fn releases_compositor_selection_on_sigterm() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    desktop.until_pixel((300, 200), |p| p == [24, 24, 32])?;
    let status = std::process::Command::new("kill")
        .args(["-TERM", &desktop.compositor.0.id().to_string()])
        .status()?;
    assert!(status.success());
    assert!(desktop.compositor.0.wait()?.success());
    let atom = desktop
        .conn
        .intern_atom(false, b"_NET_WM_CM_S0")?
        .reply()?
        .atom;
    assert_eq!(desktop.conn.get_selection_owner(atom)?.reply()?.owner, NONE);
    Ok(())
}
