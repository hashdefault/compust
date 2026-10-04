use crate::{
    rect,
    reload::half_red,
    rules::{atom, frame, set_types},
    support::{Desktop, requests::Requests},
};
use anyhow::{Context as _, Result};
use x11rb::{
    NONE,
    connection::Connection as _,
    protocol::xproto::{
        AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, GET_PROPERTY_REQUEST,
        PropMode, Window, WindowClass,
    },
    wrapper::ConnectionExt as _,
};

const RED: [u8; 3] = [255, 0, 0];
/// Unfocused windows at half opacity, without fades or blur.
const DIM: &str = "fade_ms = 0\nblur_radius = 0\n\n[[rules]]\nfocused = false\nopacity = 50\n";

/// Report `window` as the active one, as a window manager does; `NONE` reports that none is.
fn activate(desktop: &Desktop, window: Window) -> Result<()> {
    let property = atom(desktop, b"_NET_ACTIVE_WINDOW")?;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            desktop.root,
            property,
            AtomEnum::WINDOW,
            &[window],
        )?
        .check()?;
    Ok(())
}

#[test]
fn rules_for_unfocused_windows_follow_the_active_window() -> Result<()> {
    let desktop = Desktop::new(DIM)?;
    let left = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let right = desktop.window(rect(160, 20), 0x00ff_0000)?;
    desktop.map(left)?;
    desktop.map(right)?;
    let (at_left, at_right) = ((40, 40), (180, 40));
    // Given a window manager that reports no active window at all, every window counts as
    // focused: a rule for unfocused windows must not dim the whole desktop.
    desktop.until_pixel(at_left, |p| p == RED)?;
    desktop.until_pixel(at_right, |p| p == RED)?;
    // The active window stays as it is and the other one dims; the rule follows the focus.
    activate(&desktop, left)?;
    desktop.until_pixel(at_right, half_red)?;
    assert_eq!(desktop.pixel(at_left)?, RED);
    activate(&desktop, right)?;
    desktop.until_pixel(at_left, half_red)?;
    desktop.until_pixel(at_right, |p| p == RED)?;
    // A property read again on the active window, here its title, leaves it focused.
    desktop
        .conn
        .change_property8(
            PropMode::REPLACE,
            right,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"t",
        )?
        .check()?;
    desktop.wait_vblanks(3)?;
    assert_eq!(desktop.pixel(at_right)?, RED);
    assert!(half_red(desktop.pixel(at_left)?));
    // A window mapped while another is active starts unfocused.
    let late = desktop.window(rect(20, 130), 0x00ff_0000)?;
    desktop.map(late)?;
    desktop.until_pixel((40, 150), half_red)?;
    // With no window active, as on an empty workspace, all are unfocused.
    activate(&desktop, NONE)?;
    desktop.until_pixel(at_right, half_red)?;
    assert!(half_red(desktop.pixel(at_left)?));
    // A property that is not a window, or none, says nothing about focus.
    let property = atom(&desktop, b"_NET_ACTIVE_WINDOW")?;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            desktop.root,
            property,
            AtomEnum::CARDINAL,
            &[left],
        )?
        .check()?;
    desktop.until_pixel(at_left, |p| p == RED)?;
    desktop.until_pixel(at_right, |p| p == RED)?;
    activate(&desktop, left)?;
    desktop.until_pixel(at_right, half_red)?;
    desktop
        .conn
        .delete_property(desktop.root, property)?
        .check()?;
    desktop.until_pixel(at_right, |p| p == RED)?;
    desktop.until_pixel((40, 150), |p| p == RED)?;
    Ok(())
}

#[test]
fn focus_combines_with_other_selectors_and_keeps_through_resizes() -> Result<()> {
    // Given a rule for unfocused normal windows and one that keeps the focused window opaque
    // under a global opacity, each window gets what its type and focus choose.
    let settings = "fade_ms = 0\nblur_radius = 0\n\n[[rules]]\nwindow_type = \"normal\"\nfocused = false\nopacity = 50\n";
    let desktop = Desktop::new(settings)?;
    let normal = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let dialog = desktop.window(rect(160, 20), 0x00ff_0000)?;
    let other = desktop.window(rect(20, 130), 0x00ff_0000)?;
    set_types(
        &desktop,
        dialog,
        &[atom(&desktop, b"_NET_WM_WINDOW_TYPE_DIALOG")?],
    )?;
    for window in [normal, dialog, other] {
        desktop.map(window)?;
    }
    activate(&desktop, other)?;
    desktop.until_pixel((40, 40), half_red)?;
    assert_eq!(desktop.pixel((180, 40))?, RED);
    assert_eq!(desktop.pixel((40, 150))?, RED);
    // A resize captures the window again and keeps its focus, lost or held.
    let wider = ConfigureWindowAux::new().width(120);
    desktop.conn.configure_window(normal, &wider)?.check()?;
    desktop.until_pixel((130, 40), half_red)?;
    desktop.conn.configure_window(other, &wider)?.check()?;
    desktop.until_pixel((130, 150), |p| p == RED)?;
    // Reloaded rules read the focus the windows already have.
    desktop.reload(
        "fade_ms = 0\nblur_radius = 0\nopacity = 50\n\n[[rules]]\nfocused = true\nopacity = 100\n",
    )?;
    desktop.until_pixel((180, 40), half_red)?;
    assert!(half_red(desktop.pixel((40, 40))?));
    assert_eq!(desktop.pixel((40, 150))?, RED);
    Ok(())
}

#[test]
fn a_frame_is_focused_when_its_client_is_active() -> Result<()> {
    let desktop = Desktop::new(DIM)?;
    // Given a client inside a window manager's frame, the window manager names the client.
    let outer = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = desktop.window(rect(0, 0), 0x00ff_0000)?;
    frame(&desktop, outer, client, true)?;
    let other = desktop.window(rect(160, 20), 0x00ff_0000)?;
    desktop.map(other)?;
    activate(&desktop, client)?;
    desktop.until_pixel((180, 40), half_red)?;
    assert_eq!(desktop.pixel((25, 25))?, RED);
    activate(&desktop, other)?;
    desktop.until_pixel((25, 25), half_red)?;
    desktop.until_pixel((180, 40), |p| p == RED)?;
    // A frame shown before its client is managed takes the focus once it finds the client.
    let late = desktop.window(rect(20, 130), 0x00ff_0000)?;
    let inner = desktop.window(rect(0, 0), 0x00ff_0000)?;
    frame(&desktop, late, inner, false)?;
    activate(&desktop, inner)?;
    desktop.until_pixel((180, 40), half_red)?;
    assert!(half_red(desktop.pixel((25, 135))?));
    let state = atom(&desktop, b"WM_STATE")?;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, inner, state, state, &[1, 0])?
        .check()?;
    desktop.until_pixel((25, 135), |p| p == RED)?;
    Ok(())
}

#[test]
fn windows_open_before_the_compositor_starts_get_their_focus() -> Result<()> {
    // Given two windows and an active one before Compust starts, its first frame already
    // dims the other. The client that made the windows stays connected for the test.
    let (desktop, _client) = Desktop::with_display(DIM, |display| {
        let (conn, screen) = x11rb::connect(Some(display))?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .context("missing screen")?
            .root;
        let mut windows = [0; 2];
        for (window, x) in windows.iter_mut().zip([20, 160]) {
            *window = conn.generate_id()?;
            let aux = CreateWindowAux::new().background_pixel(0x00ff_0000);
            conn.create_window(
                24,
                *window,
                root,
                x,
                20,
                100,
                100,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &aux,
            )?
            .check()?;
            conn.map_window(*window)?.check()?;
        }
        let property = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
            .reply()?
            .atom;
        conn.change_property32(
            PropMode::REPLACE,
            root,
            property,
            AtomEnum::WINDOW,
            &windows[..1],
        )?
        .check()?;
        Ok((display.to_owned(), conn))
    })?;
    desktop.until_pixel((180, 40), half_red)?;
    assert_eq!(desktop.pixel((40, 40))?, RED);
    Ok(())
}

#[test]
fn focus_outlasts_a_suspension_of_compositing() -> Result<()> {
    let desktop = Desktop::new(&format!("unredirect_fullscreen = true\n{DIM}"))?;
    let dim = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let cover = desktop.window(crate::regions::area(0, 0, 320, 240), 0x0000_00ff)?;
    desktop.map(dim)?;
    activate(&desktop, cover)?;
    desktop.until_pixel((40, 40), half_red)?;
    // Given compositing suspended behind the active window and resumed when it goes, the
    // window captured again is still unfocused.
    desktop.map(cover)?;
    desktop.until_suspended(true)?;
    desktop.conn.unmap_window(cover)?.check()?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((40, 40), half_red)?;
    desktop.wait_vblanks(3)?;
    assert!(half_red(desktop.pixel((40, 40))?));
    Ok(())
}

#[test]
fn a_focus_change_reads_one_property() -> Result<()> {
    let (desktop, mut requests) = Desktop::with_display(DIM, Requests::start)?;
    let windows = [
        desktop.window(rect(20, 20), 0x00ff_0000)?,
        desktop.window(rect(160, 20), 0x00ff_0000)?,
        desktop.window(rect(20, 130), 0x00ff_0000)?,
    ];
    for window in windows {
        desktop.map(window)?;
    }
    let [first, second, _] = windows;
    activate(&desktop, first)?;
    desktop.until_pixel((180, 40), half_red)?;
    desktop.wait_vblanks(2)?;
    let before = requests.count(GET_PROPERTY_REQUEST);
    // Given three windows, moving the focus reads the root's property and nothing from them.
    activate(&desktop, second)?;
    desktop.until_pixel((40, 40), half_red)?;
    desktop.until_pixel((180, 40), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    assert_eq!(requests.count(GET_PROPERTY_REQUEST) - before, 1);
    requests.finish()
}
