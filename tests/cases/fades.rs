use crate::{rect, support::Desktop};
use anyhow::Result;
use std::time::Instant;
use x11rb::protocol::xproto::*;

#[test]
fn closes_window_destroyed_during_opening() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |[r, _, _]| (80..140).contains(&r))?;

    desktop.conn.destroy_window(window)?.check()?;

    desktop.until_pixel((40, 40), |[r, _, _]| (40..70).contains(&r))?;
    desktop.screenshot("fade-interrupted-close")?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn resumes_opacity_and_recaptures_content_when_remapped_during_close() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.conn.unmap_window(window)?.check()?;
    let started = Instant::now();
    let before = desktop.until_pixel((40, 40), |[r, _, _]| (160..200).contains(&r))?;
    let observed = Instant::now();

    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().background_pixel(0x0000_ff00),
        )?
        .check()?;
    desktop.map(window)?;

    let after = desktop.until_pixel((40, 40), |[_, g, _]| g > 24)?;
    // Allow one 120 Hz frame and rounding, plus the maximum smoothstep slope.
    let allowance = u8::try_from((6 + observed.elapsed().as_millis() * 383 / 1000).min(255))?;
    let [red, _, _] = before;
    let [_, green, _] = after;
    assert!(
        green >= red.saturating_sub(allowance),
        "remap restarted opacity: {before:?} -> {after:?}, elapsed {:?}, closing {:?}",
        observed.elapsed(),
        started.elapsed()
    );
    desktop.screenshot("fade-remapped-content")?;
    desktop.until_pixel((40, 40), |p| p == [0, 255, 0])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn preserves_stacking_of_destroyed_window_during_fade() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let upper = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(upper)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    let marker = desktop.window(rect(180, 20), 0x0000_ff00)?;
    desktop.screenshot("fade-destroyed-stacking-before")?;

    desktop.conn.grab_server()?.check()?;
    desktop.conn.destroy_window(lower)?.check()?;
    desktop.map(marker)?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((200, 40), |[_, g, _]| (40..100).contains(&g))?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
    desktop.screenshot("fade-destroyed-stacking")?;
    desktop.until_pixel((200, 40), |p| p == [0, 255, 0])?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
    desktop.screenshot("fade-destroyed-stacking-settled")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

/// Two tiled windows, the right one above the left in the stack, when `close` takes the left
/// one away and the right one grows over its place at once, as a tiling manager does: the
/// closing window keeps fading over the neighbor instead of vanishing under it.
fn closes_beside_a_neighbor(
    name: &str,
    close: impl FnOnce(&Desktop, Window) -> Result<()>,
) -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let left = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let right = desktop.window(rect(120, 20), 0x0000_00ff)?;
    desktop.map(left)?;
    desktop.map(right)?;
    desktop.until_pixel((60, 60), |p| p == [255, 0, 0])?;
    desktop.until_pixel((160, 60), |p| p == [0, 0, 255])?;

    desktop.conn.grab_server()?.check()?;
    close(&desktop, left)?;
    desktop
        .conn
        .configure_window(right, &ConfigureWindowAux::new().x(20).width(200))?
        .check()?;
    desktop.conn.ungrab_server()?.check()?;

    let [red, _, blue] = desktop.until_pixel((60, 60), |[r, _, b]| r > 40 && b > 40)?;
    desktop.screenshot(name)?;
    assert!(
        red > 40 && blue > 40,
        "closing window under its neighbor: {red} {blue}"
    );
    desktop.until_pixel((60, 60), |p| p == [0, 0, 255])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn destroyed_window_fades_over_the_neighbor_that_takes_its_place() -> Result<()> {
    closes_beside_a_neighbor("fade-destroyed-over-neighbor", |desktop, window| {
        desktop.conn.destroy_window(window)?.check()?;
        Ok(())
    })
}

#[test]
fn unmapped_window_fades_over_the_neighbor_that_takes_its_place() -> Result<()> {
    closes_beside_a_neighbor("fade-unmapped-over-neighbor", |desktop, window| {
        desktop.conn.unmap_window(window)?.check()?;
        Ok(())
    })
}
