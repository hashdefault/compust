use crate::{
    rect,
    support::{Desktop, next_event, presentation::Presentation, requests::Requests},
};
use anyhow::Result;
use std::time::{Duration, Instant};
use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        present::{CompleteKind, ConnectionExt as _, EventMask},
        xproto::{
            ConfigureWindowAux, ConnectionExt as _, GET_GEOMETRY_REQUEST, QUERY_TREE_REQUEST,
            StackMode,
        },
    },
};

#[test]
fn moves_and_restacks_need_no_tree_or_geometry_queries() -> Result<()> {
    let (mut desktop, mut requests) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Requests::start)?;
    let red = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let blue = desktop.window(rect(60, 60), 0x0000_00ff)?;
    let green = desktop.window(rect(100, 100), 0x0000_ff00)?;
    for window in [red, blue, green] {
        desktop.map(window)?;
    }
    desktop.until_pixel((110, 110), |p| p == [0, 255, 0])?;
    let trees = requests.count(QUERY_TREE_REQUEST);
    let geometries = requests.count(GET_GEOMETRY_REQUEST);

    // Drag one window while the others trade places, as a window manager does.
    for step in 0..=16 {
        desktop
            .conn
            .configure_window(blue, &ConfigureWindowAux::new().x(60 + step))?;
        let restack = if step % 2 == 0 {
            ConfigureWindowAux::new()
                .sibling(green)
                .stack_mode(StackMode::ABOVE)
        } else {
            ConfigureWindowAux::new().stack_mode(StackMode::BELOW)
        };
        desktop.conn.configure_window(red, &restack)?;
    }
    desktop.conn.flush()?;

    // Red ends above green, which stays above blue.
    desktop.until_pixel((110, 110), |p| p == [255, 0, 0])?;
    desktop.until_pixel((140, 140), |p| p == [0, 255, 0])?;
    desktop.until_pixel((90, 150), |p| p == [0, 0, 255])?;
    assert_eq!(
        requests.count(QUERY_TREE_REQUEST) - trees,
        0,
        "moves and restacks queried the window tree"
    );
    assert_eq!(
        requests.count(GET_GEOMETRY_REQUEST) - geometries,
        0,
        "moves queried window geometry"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    requests.finish()
}

#[test]
fn events_that_change_nothing_on_screen_do_not_repaint() -> Result<()> {
    let (desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let shown = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let hidden = desktop.window(rect(160, 20), 0x0000_00ff)?;
    desktop.map(shown)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.conn.unmap_window(shown)?.check()?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    wait_vblanks(&desktop, 2)?;
    let before = presentation.submissions();

    // An extra frame here would delay the next real one by a vblank.
    desktop.conn.destroy_window(shown)?.check()?;
    desktop
        .conn
        .configure_window(hidden, &ConfigureWindowAux::new().x(180))?
        .check()?;
    wait_vblanks(&desktop, 4)?;

    assert_eq!(
        presentation.submissions(),
        before,
        "an invisible change was repainted"
    );
    desktop.map(hidden)?;
    desktop.until_pixel((200, 40), |p| p == [0, 0, 255])?;
    assert!(presentation.submissions() > before);
    presentation.finish()
}

/// Wait until Present reports `count` more vblanks on the overlay's CRTC.
fn wait_vblanks(desktop: &Desktop, count: u64) -> Result<()> {
    let id = desktop.conn.generate_id()?;
    desktop
        .conn
        .present_select_input(id, desktop.overlay, EventMask::COMPLETE_NOTIFY)?
        .check()?;
    desktop
        .conn
        .present_notify_msc(desktop.overlay, 1, 0, 0, 0)?
        .check()?;
    let msc = notified(desktop, id, 1)?;
    desktop
        .conn
        .present_notify_msc(desktop.overlay, 2, msc + count, 0, 0)?
        .check()?;
    notified(desktop, id, 2)?;
    desktop
        .conn
        .present_select_input(id, desktop.overlay, EventMask::from(0_u32))?
        .check()?;
    Ok(())
}

fn notified(desktop: &Desktop, id: u32, serial: u32) -> Result<u64> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Event::PresentCompleteNotify(event) = next_event(&desktop.conn, deadline)?
            && event.event == id
            && event.serial == serial
            && event.kind == CompleteKind::NOTIFY_MSC
        {
            return Ok(event.msc);
        }
    }
}
