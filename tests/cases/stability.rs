use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::protocol::{
    shape::{ConnectionExt as _, SK, SO},
    xproto::*,
};

#[test]
fn tracks_rapid_map_unmap_and_remap() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.until_pixel((180, 40), |p| p == [24, 24, 32])?;
    desktop.screenshot("stability-remap-before")?;

    desktop.conn.grab_server()?.check()?;
    for _ in 0..32 {
        desktop.conn.unmap_window(window)?.check()?;
        desktop.map(window)?;
    }
    desktop.conn.unmap_window(window)?.check()?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(160))?
        .check()?;
    desktop.map(window)?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    desktop.until_pixel((180, 40), |p| p == [255, 0, 0])?;
    desktop.screenshot("stability-remap-after")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn removes_windows_destroyed_with_queued_property_configure_and_shape_events() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let doomed = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(doomed)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.screenshot("stability-queued-destroy-before")?;

    desktop.conn.grab_server()?.check()?;
    desktop.opacity(doomed, 0x8000_0000)?;
    desktop
        .conn
        .configure_window(doomed, &ConfigureWindowAux::new().width(120))?
        .check()?;
    desktop
        .conn
        .shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            doomed,
            0,
            0,
            &[rect(0, 0)],
        )?
        .check()?;
    desktop.conn.destroy_window(doomed)?.check()?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.screenshot("stability-queued-destroy-after")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn restores_pixels_beneath_removed_override_redirect_popup() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(lower)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    let popup = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop
        .conn
        .change_window_attributes(
            popup,
            &ChangeWindowAttributesAux::new().override_redirect(1),
        )?
        .check()?;
    desktop.map(popup)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.screenshot("stability-popup-before")?;

    desktop.conn.destroy_window(popup)?.check()?;

    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.screenshot("stability-popup-after")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn restores_geometry_after_fullscreen_sized_resize() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.until_pixel((300, 200), |p| p == [24, 24, 32])?;
    desktop.screenshot("stability-fullscreen-before")?;

    desktop
        .conn
        .configure_window(
            window,
            &ConfigureWindowAux::new().x(0).y(0).width(320).height(240),
        )?
        .check()?;
    desktop.until_pixel((300, 200), |p| p == [255, 0, 0])?;
    desktop.until_pixel((5, 5), |p| p == [255, 0, 0])?;
    desktop.screenshot("stability-fullscreen-expanded")?;
    desktop
        .conn
        .configure_window(
            window,
            &ConfigureWindowAux::new().x(20).y(20).width(100).height(100),
        )?
        .check()?;

    desktop.until_pixel((300, 200), |p| p == [24, 24, 32])?;
    desktop.until_pixel((5, 5), |p| p == [24, 24, 32])?;
    assert_eq!(desktop.pixel((40, 40))?, [255, 0, 0]);
    desktop.screenshot("stability-fullscreen-restored")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
