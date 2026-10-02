use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};

fn child(desktop: &Desktop, parent: Window) -> Result<Window> {
    let window = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            24,
            window,
            parent,
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
    Ok(window)
}

fn wm_state(desktop: &Desktop, window: Window) -> Result<Atom> {
    let atom = desktop.conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, window, atom, atom, &[1, 0])?
        .check()?;
    Ok(atom)
}

#[test]
fn tracks_late_wm_state_and_removal_with_frame_opacity_precedence() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = child(&desktop, frame)?;
    desktop.opacity(client, 0)?;
    desktop.map(client)?;
    desktop.map(frame)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    let state = wm_state(&desktop, client)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    desktop.conn.delete_property(client, state)?.check()?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    wm_state(&desktop, client)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;

    desktop.opacity(frame, u32::MAX)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    let opacity = desktop
        .conn
        .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
        .reply()?
        .atom;
    desktop.conn.delete_property(frame, opacity)?.check()?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn discovers_clients_created_below_existing_frames() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let intermediate = child(&desktop, frame)?;
    desktop.map(intermediate)?;
    desktop.map(frame)?;
    desktop.until_pixel((50, 50), |p| p == [0, 0, 255])?;

    let client = child(&desktop, intermediate)?;
    wm_state(&desktop, client)?;
    desktop.opacity(client, 0)?;
    desktop.map(client)?;
    desktop.until_pixel((50, 50), |p| p == [24, 24, 32])?;
    desktop.opacity(client, u32::MAX)?;
    desktop.until_pixel((50, 50), |p| p == [0, 0, 255])?;
    Ok(())
}

#[test]
fn tracks_clients_reparented_between_mapped_frames_and_root() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let first = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let second = desktop.window(rect(160, 20), 0x0000_ff00)?;
    let client = child(&desktop, first)?;
    wm_state(&desktop, client)?;
    desktop.map(client)?;
    desktop.map(first)?;
    desktop.map(second)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.until_pixel((180, 40), |p| p == [0, 255, 0])?;
    desktop.opacity(client, 0)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;

    desktop
        .conn
        .reparent_window(client, second, 10, 10)?
        .check()?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.until_pixel((180, 40), |p| p == [24, 24, 32])?;
    desktop.opacity(client, u32::MAX)?;
    desktop.until_pixel((180, 40), |p| p == [0, 0, 255])?;

    desktop
        .conn
        .reparent_window(client, desktop.root, 20, 160)?
        .check()?;
    desktop.until_pixel((180, 40), |p| p == [0, 255, 0])?;
    desktop.until_pixel((40, 180), |p| p == [0, 0, 255])?;
    desktop.conn.destroy_window(client)?.check()?;
    desktop.until_pixel((40, 180), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn restores_frame_opacity_when_client_is_destroyed() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = child(&desktop, frame)?;
    wm_state(&desktop, client)?;
    desktop.map(client)?;
    desktop.map(frame)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.opacity(client, 0)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;

    desktop.conn.destroy_window(client)?.check()?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    Ok(())
}

#[test]
fn keeps_frame_visible_when_client_dies_with_queued_opacity_events() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = child(&desktop, frame)?;
    wm_state(&desktop, client)?;
    desktop.map(client)?;
    desktop.map(frame)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    desktop.conn.grab_server()?.check()?;
    desktop.opacity(client, 0)?;
    desktop.conn.destroy_window(client)?.check()?;
    desktop.conn.ungrab_server()?.check()?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    Ok(())
}
