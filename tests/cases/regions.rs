use crate::{
    rect,
    support::{
        Desktop,
        monitors::Monitor,
        presentation::{Area, Presentation},
    },
};
use anyhow::{Context, Result};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, Rectangle, StackMode, WindowClass,
    },
};

const BACKGROUND: [u8; 3] = [24, 24, 32];
const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];

fn area(x: i16, y: i16, width: u16, height: u16) -> Rectangle {
    Rectangle {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn small_damage_presents_only_its_area() -> Result<()> {
    let (desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(area(20, 20, 200, 150), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let before = presentation.updates()?.len();

    desktop.fill(window, area(50, 40, 10, 10), 0x0000_00ff)?;
    desktop.until_pixel((75, 65), |p| p == BLUE)?;
    // Idle frames would show up as further submissions.
    desktop.wait_vblanks(4)?;

    let updates = presentation.updates()?;
    let after = updates.get(before..).context("missing submissions")?;
    assert_eq!(after, [Some(vec![(70, 60, 10, 10)])]);
    for (point, expected) in [
        ((70, 60), BLUE),
        ((79, 69), BLUE),
        ((69, 60), RED),
        ((80, 69), RED),
        ((70, 59), RED),
        ((79, 70), RED),
    ] {
        assert_eq!(desktop.pixel(point)?, expected, "at {point:?}");
    }
    presentation.finish()
}

#[test]
fn moves_and_restacks_repaint_only_what_they_change() -> Result<()> {
    let (desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let back = desktop.window(rect(20, 20), 0x0000_ff00)?;
    let front = desktop.window(rect(60, 60), 0x0000_00ff)?;
    let far = desktop.window(area(240, 20, 60, 60), 0x00ff_0000)?;
    for window in [back, front, far] {
        desktop.map(window)?;
    }
    desktop.until_pixel((100, 100), |p| p == BLUE)?;
    desktop.until_pixel((250, 30), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let start = presentation.updates()?.len();

    desktop
        .conn
        .configure_window(
            back,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )?
        .check()?;
    desktop.until_pixel((100, 100), |p| p == GREEN)?;
    desktop.wait_vblanks(2)?;
    assert_eq!(desktop.pixel((140, 140))?, BLUE);
    let raised = presentation.updates()?.len();

    desktop
        .conn
        .configure_window(front, &ConfigureWindowAux::new().x(150).y(120))?
        .check()?;
    desktop.until_pixel((200, 170), |p| p == BLUE)?;
    desktop.wait_vblanks(2)?;
    assert_eq!(desktop.pixel((140, 140))?, BACKGROUND);
    assert_eq!(desktop.pixel((100, 100))?, GREEN);
    assert_eq!(desktop.pixel((250, 30))?, RED);

    let updates = presentation.updates()?;
    let restack = updates
        .get(start..raised)
        .context("missing restack frames")?;
    let movement = updates.get(raised..).context("missing move frames")?;
    assert!(!restack.is_empty() && !movement.is_empty());
    assert!(
        confined(restack, &[(60, 60, 60, 60)]),
        "restack repainted {restack:?}"
    );
    assert!(
        confined(movement, &[(60, 60, 100, 100), (150, 120, 100, 100)]),
        "move repainted {movement:?}"
    );
    presentation.finish()
}

#[test]
fn fades_repaint_only_the_fading_window() -> Result<()> {
    let (desktop, mut presentation) =
        Desktop::with_display("fade_ms = 300\nblur_radius = 0", Presentation::start)?;
    let still = desktop.window(area(240, 20, 60, 60), 0x00ff_0000)?;
    desktop.map(still)?;
    desktop.until_pixel((250, 30), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let before = presentation.updates()?.len();

    let fading = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(fading)?;
    desktop.until_pixel((50, 50), |p| p == BLUE)?;
    desktop.conn.unmap_window(fading)?.check()?;
    desktop.until_pixel((50, 50), |p| p == BACKGROUND)?;
    desktop.wait_vblanks(2)?;

    let updates = presentation.updates()?;
    let fades = updates.get(before..).context("missing fade frames")?;
    assert!(fades.len() > 4, "two fades took {} frames", fades.len());
    assert!(
        confined(fades, &[(20, 20, 100, 100)]),
        "fades repainted {fades:?}"
    );
    assert_eq!(desktop.pixel((250, 30))?, RED);
    presentation.finish()
}

#[test]
fn blurred_damage_matches_a_full_repaint_with_present() -> Result<()> {
    blurred_damage_matches_a_full_repaint(true)
}

#[test]
fn blurred_damage_matches_a_full_repaint_with_xrender() -> Result<()> {
    blurred_damage_matches_a_full_repaint(false)
}

/// Blur reads around each pixel it writes, so damage beside or under a blurred window must
/// repaint the window's whole footprint, and through it any footprint that one overlaps.
fn blurred_damage_matches_a_full_repaint(vsync: bool) -> Result<()> {
    let mut desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = 4\nvsync = {vsync}"))?;
    let background = striped(&desktop, 320)?;
    // With radius 4 the footprints span x 92..208 and 196..276.
    let left = desktop.window(rect(100, 60), 0)?;
    let right = desktop.window(area(205, 60, 60, 60), 0)?;
    for window in [left, right] {
        desktop.opacity(window, 0x8000_0000)?;
        desktop.map(window)?;
    }
    desktop.until_pixel((230, 90), |[r, g, b]| r < 200 && r == g && g == b)?;
    desktop.wait_vblanks(2)?;

    // Beside the left window, within its footprint.
    desktop.fill(background, area(94, 100, 3, 3), 0x00ff_0000)?;
    desktop.until_pixel((95, 101), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop)?, "beside a blur");

    // Under the right window only; its footprint reaches the left window's.
    desktop.fill(background, area(240, 100, 3, 3), 0x00ff_0000)?;
    desktop.wait_vblanks(4)?;
    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop)?, "under a chained blur");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn monitor_changes_repaint_the_whole_screen() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 4")?;
    // The right half shows the bare background, which no window's changes repaint.
    striped(&desktop, 160)?;
    let moving = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let edge = desktop.window(area(200, 150, 100, 80), 0x0000_00ff)?;
    desktop.opacity(edge, 0x8000_0000)?;
    for window in [moving, edge] {
        desktop.map(window)?;
    }
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.wait_vblanks(2)?;

    let monitor = Monitor::new(&desktop)?;
    monitor.disable(&desktop)?;
    desktop.resize_root((240, 180))?;
    desktop
        .conn
        .configure_window(moving, &ConfigureWindowAux::new().x(120))?
        .check()?;
    desktop.until_pixel((130, 30), |p| p == RED)?;
    desktop.resize_root((320, 240))?;
    monitor.restore(&desktop)?;
    desktop.until_pixel((60, 30), |p| p != RED)?;
    desktop.wait_vblanks(2)?;

    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop)?, "after a monitor change");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn overlay_exposures_repaint_the_exposed_area() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.wait_vblanks(2)?;

    // Screen lockers such as xsecurelock draw in a child of the overlay, above every window.
    let lock = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            24,
            lock,
            desktop.overlay,
            0,
            0,
            320,
            240,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().background_pixel(0x00ff_ff00),
        )?
        .check()?;
    desktop.map(lock)?;
    desktop.until_pixel((50, 50), |p| p == [255, 255, 0])?;
    desktop.conn.destroy_window(lock)?.check()?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.until_pixel((200, 200), |p| p == BACKGROUND)?;
    Ok(())
}

/// A window at the left of the screen with one-pixel black and white stripes, which any
/// blur changes.
fn striped(desktop: &Desktop, width: u16) -> Result<u32> {
    let background = desktop.window(area(0, 0, width, 240), 0x00ff_ffff)?;
    desktop.map(background)?;
    for x in (0..width.cast_signed()).step_by(2) {
        desktop.fill(background, area(x, 0, 1, 240), 0)?;
    }
    desktop.until_pixel((10, 10), |p| p == [0, 0, 0])?;
    Ok(background)
}

/// Cover the screen and uncover it, so the next frame repaints everything, and return it.
fn full_repaint(desktop: &Desktop) -> Result<Vec<u8>> {
    let cover = desktop.window(area(0, 0, 320, 240), 0x00ff_00ff)?;
    desktop.map(cover)?;
    desktop.until_pixel((0, 0), |p| p == [255, 0, 255])?;
    desktop.conn.destroy_window(cover)?.check()?;
    desktop.until_pixel((0, 0), |p| p != [255, 0, 255])?;
    desktop.wait_vblanks(2)?;
    desktop.image()
}

fn assert_same(shown: &[u8], expected: &[u8], case: &str) {
    let differing: Vec<_> = shown
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| (index % 320, index / 320))
        .collect();
    assert!(
        differing.is_empty(),
        "{case}: {} pixels differ from a full repaint, first at {:?}",
        differing.len(),
        differing.first()
    );
    assert_eq!(shown.len(), expected.len());
}

/// Whether every submission updated only rectangles inside one of `allowed`.
fn confined(updates: &[Option<Area>], allowed: &[(i16, i16, u16, u16)]) -> bool {
    let inside = |(x, y, width, height): (i16, i16, u16, u16),
                  (left, top, span, depth): (i16, i16, u16, u16)| {
        x >= left
            && y >= top
            && i32::from(x) + i32::from(width) <= i32::from(left) + i32::from(span)
            && i32::from(y) + i32::from(height) <= i32::from(top) + i32::from(depth)
    };
    updates.iter().all(|update| {
        update.as_ref().is_some_and(|rects| {
            rects
                .iter()
                .all(|rect| allowed.iter().any(|bound| inside(*rect, *bound)))
        })
    })
}
