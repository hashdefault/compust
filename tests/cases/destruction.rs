use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::protocol::{
    shape::{ConnectionExt as _, SK, SO},
    xproto::*,
};

#[test]
fn survives_windows_destroyed_before_capture() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    let survivor = desktop.window(rect(180, 20), 0x0000_ff00)?;

    desktop.conn.grab_server()?.check()?;
    for _ in 0..32 {
        let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
        desktop.map(window)?;
        desktop.conn.destroy_window(window)?.check()?;
    }
    desktop.map(survivor)?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((200, 40), |p| p == [0, 255, 0])?;
    assert_eq!(desktop.pixel((40, 40))?, [24, 24, 32]);
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn survives_destruction_before_shape_refresh() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let doomed = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(doomed)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    desktop.conn.grab_server()?.check()?;
    desktop
        .conn
        .shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            doomed,
            0,
            0,
            &[Rectangle {
                x: 0,
                y: 0,
                width: 50,
                height: 100,
            }],
        )?
        .check()?;
    desktop.conn.destroy_window(doomed)?.check()?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn survives_destruction_before_geometry_refresh() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let doomed = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(doomed)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    desktop.conn.grab_server()?.check()?;
    desktop
        .conn
        .configure_window(doomed, &ConfigureWindowAux::new().width(180))?
        .check()?;
    desktop.conn.destroy_window(doomed)?.check()?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    assert_eq!(desktop.pixel((180, 40))?, [24, 24, 32]);
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
