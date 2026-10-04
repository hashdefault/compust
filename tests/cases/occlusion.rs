use crate::{
    rect,
    regions::{area, assert_same, full_repaint, striped},
    support::{Desktop, requests::Requests},
};
use anyhow::{Context, Result};
use x11rb::{
    connection::RequestConnection,
    protocol::{
        render::{self, COMPOSITE_REQUEST},
        shape::{ConnectionExt as _, SK, SO},
        xproto::{ClipOrdering, ConfigureWindowAux, ConnectionExt as _, Rectangle},
    },
};

#[test]
fn windows_under_an_opaque_one_are_not_painted() -> Result<()> {
    let (desktop, mut requests) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 4", Requests::start)?;
    let render = desktop
        .conn
        .extension_information(render::X11_EXTENSION_NAME)?
        .context("missing RENDER")?
        .major_opcode;
    for (index, x) in [20, 60, 100].into_iter().enumerate() {
        let window = desktop.window(rect(x, 40), 0x0000_ff00)?;
        if index == 1 {
            desktop.opacity(window, 0x8000_0000)?;
        }
        desktop.map(window)?;
    }
    let top = desktop.window(area(0, 0, 320, 240), 0x00ff_0000)?;
    desktop.map(top)?;
    desktop.until_pixel((50, 50), |p| p == [255, 0, 0])?;
    desktop.wait_vblanks(2)?;
    let before = requests.count_extension(render, COMPOSITE_REQUEST)?;

    // The whole top window changes; neither the windows beneath it nor the blurred one's
    // backdrop are composited again.
    desktop.fill(top, area(0, 0, 320, 240), 0x0000_00ff)?;
    desktop.until_pixel((50, 50), |p| p == [0, 0, 255])?;
    desktop.wait_vblanks(2)?;
    assert_eq!(
        requests.count_extension(render, COMPOSITE_REQUEST)? - before,
        1,
        "a frame of the top window composited more than it"
    );
    requests.finish()
}

#[test]
fn hidden_windows_reappear_with_every_change_to_what_hides_them() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4";
    let mut desktop = Desktop::new(settings)?;
    striped(&desktop, 320)?;
    let blurred = desktop.window(rect(60, 60), 0x0000_0080)?;
    desktop.opacity(blurred, 0x8000_0000)?;
    let plain = desktop.window(area(140, 100, 60, 60), 0x0000_ff00)?;
    let cover = desktop.window(area(40, 40, 200, 150), 0x00ff_0000)?;
    for window in [blurred, plain, cover] {
        desktop.map(window)?;
    }
    desktop.until_pixel((150, 110), |p| p == [255, 0, 0])?;
    desktop.wait_vblanks(2)?;
    let step = |case: &str, change: &dyn Fn() -> Result<()>| {
        change()?;
        desktop.wait_vblanks(4)?;
        let shown = desktop.image()?;
        assert_same(&shown, &full_repaint(&desktop, settings)?, case);
        anyhow::Ok(())
    };
    let configure = |aux: &ConfigureWindowAux| -> Result<()> {
        desktop.conn.configure_window(cover, aux)?.check()?;
        Ok(())
    };
    step("the cover moved aside", &|| {
        configure(&ConfigureWindowAux::new().x(150))
    })?;
    step("the cover moved back", &|| {
        configure(&ConfigureWindowAux::new().x(40))
    })?;
    step("the cover turned translucent", &|| {
        desktop.opacity(cover, 0x8000_0000)
    })?;
    step("the cover turned opaque", &|| {
        desktop.opacity(cover, u32::MAX)
    })?;
    step("the cover took a hole", &|| {
        let ring = [
            area(0, 0, 200, 30),
            area(0, 120, 200, 30),
            area(0, 30, 30, 90),
            area(170, 30, 30, 90),
        ];
        desktop
            .conn
            .shape_rectangles(
                SO::SET,
                SK::BOUNDING,
                ClipOrdering::UNSORTED,
                cover,
                0,
                0,
                &ring,
            )?
            .check()?;
        Ok(())
    })?;
    step("the cover went away", &|| {
        desktop.conn.unmap_window(cover)?.check()?;
        Ok(())
    })?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn blurs_beneath_an_opaque_window_read_what_it_hides() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4";
    let mut desktop = Desktop::new(settings)?;
    let background = striped(&desktop, 320)?;
    // The cover hides the right part of the green window, which lies under the blurred
    // window's right half and so in its footprint.
    let green = desktop.window(area(140, 80, 30, 60), 0x0000_ff00)?;
    let blurred = desktop.window(rect(100, 60), 0x0000_0080)?;
    desktop.opacity(blurred, 0x8000_0000)?;
    let cover = desktop.window(area(150, 40, 100, 160), 0x00ff_0000)?;
    for window in [green, blurred, cover] {
        desktop.map(window)?;
    }
    desktop.until_pixel((200, 100), |p| p == [255, 0, 0])?;
    desktop.wait_vblanks(2)?;

    // A change beneath the blurred window blurs it again, reading the scene under the cover.
    desktop.fill(background, area(120, 100, 4, 4), 0x00ff_0000)?;
    desktop.wait_vblanks(4)?;
    let shown = desktop.image()?;
    // Without the cover, nothing hides that scene; outside the cover the frames must match.
    desktop.conn.unmap_window(cover)?.check()?;
    let expected = full_repaint(&desktop, settings)?;
    assert_same_outside(&shown, &expected, area(150, 40, 100, 160));
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn a_blur_hidden_whole_blurs_when_uncovered() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4";
    let mut desktop = Desktop::new(settings)?;
    let background = striped(&desktop, 320)?;
    let blurred = desktop.window(rect(100, 60), 0x0000_0080)?;
    desktop.opacity(blurred, 0x8000_0000)?;
    desktop.map(blurred)?;
    desktop.until_pixel((150, 110), |[r, g, b]| b > r && b > g)?;
    desktop.wait_vblanks(2)?;
    // The window has a backdrop by now; the cover then hides it whole.
    let cover = desktop.window(area(80, 40, 140, 140), 0x00ff_0000)?;
    desktop.map(cover)?;
    desktop.until_pixel((150, 110), |p| p == [255, 0, 0])?;
    desktop.wait_vblanks(2)?;

    // The scene beneath changes while the blurred window is hidden whole; once uncovered,
    // it must not show the backdrop it had before.
    desktop.fill(background, area(140, 100, 20, 20), 0x00ff_0000)?;
    desktop.wait_vblanks(4)?;
    desktop
        .conn
        .configure_window(cover, &ConfigureWindowAux::new().x(220))?
        .check()?;
    desktop.wait_vblanks(4)?;
    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop, settings)?, "uncovered");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

/// Compare two frames everywhere but `except`.
fn assert_same_outside(shown: &[u8], expected: &[u8], except: Rectangle) {
    let inside = |index: usize| {
        let (x, y) = (index % 320, index / 320);
        let (left, top) = (
            usize::try_from(except.x).unwrap_or_default(),
            usize::try_from(except.y).unwrap_or_default(),
        );
        (left..left + usize::from(except.width)).contains(&x)
            && (top..top + usize::from(except.height)).contains(&y)
    };
    let differing: Vec<_> = shown
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
        .filter(|&(index, (left, right))| left != right && !inside(index))
        .map(|(index, _)| (index % 320, index / 320))
        .collect();
    assert!(
        differing.is_empty(),
        "{} pixels outside the cover differ, first at {:?}",
        differing.len(),
        differing.first()
    );
    assert_eq!(shown.len(), expected.len());
}
