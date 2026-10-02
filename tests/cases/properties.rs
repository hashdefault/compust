use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::{protocol::xproto::*, wrapper::ConnectionExt as _};

fn invalid_client_state(replace: impl Fn(&Desktop, Window, Atom) -> Result<()>) -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = desktop.window(rect(0, 0), 0x0000_00ff)?;
    desktop
        .conn
        .reparent_window(client, frame, 10, 10)?
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

    replace(&desktop, client, state)?;

    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, client, state, state, &[1, 0])?
        .check()?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn ignores_wm_state_with_wrong_type() -> Result<()> {
    invalid_client_state(|desktop, client, state| {
        desktop
            .conn
            .change_property32(
                PropMode::REPLACE,
                client,
                state,
                AtomEnum::CARDINAL,
                &[1, 0],
            )?
            .check()?;
        Ok(())
    })
}

#[test]
fn ignores_wm_state_with_wrong_format() -> Result<()> {
    invalid_client_state(|desktop, client, state| {
        desktop
            .conn
            .change_property8(PropMode::REPLACE, client, state, state, &[1, 0])?
            .check()?;
        Ok(())
    })
}

#[test]
fn ignores_wm_state_with_wrong_length() -> Result<()> {
    for values in [&[][..], &[1][..], &[1, 0, 0][..]] {
        invalid_client_state(|desktop, client, state| {
            desktop
                .conn
                .change_property32(PropMode::REPLACE, client, state, state, values)?
                .check()?;
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn ignores_opacity_with_extra_values() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.opacity(window, 0)?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    let opacity = desktop
        .conn
        .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
        .reply()?
        .atom;

    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            window,
            opacity,
            AtomEnum::CARDINAL,
            &[0, 0],
        )?
        .check()?;

    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    Ok(())
}

#[test]
fn falls_back_to_client_opacity_when_frame_property_is_malformed() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let frame = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = desktop.window(rect(0, 0), 0x0000_00ff)?;
    desktop
        .conn
        .reparent_window(client, frame, 10, 10)?
        .check()?;
    let state = desktop.conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, client, state, state, &[1, 0])?
        .check()?;
    desktop.opacity(client, 0)?;
    desktop.opacity(frame, u32::MAX)?;
    desktop.map(client)?;
    desktop.map(frame)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    let opacity = desktop
        .conn
        .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
        .reply()?
        .atom;

    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            frame,
            opacity,
            AtomEnum::CARDINAL,
            &[u32::MAX, 0],
        )?
        .check()?;

    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}
