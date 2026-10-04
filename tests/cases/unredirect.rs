use crate::{
    rect,
    regions::{BACKGROUND, area, assert_same, full_repaint},
    support::Desktop,
};
use anyhow::Result;
use x11rb::{
    NONE,
    connection::Connection as _,
    protocol::{
        Event,
        shape::{ConnectionExt as _, SK, SO},
        xproto::{
            ChangeWindowAttributesAux, ClipOrdering, ConfigureWindowAux, ConnectionExt as _,
            CreateWindowAux, EventMask, StackMode, Window, WindowClass,
        },
    },
};

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];
/// Unredirection on, without fades; the blur radius lets tests ask for a full repaint.
const SETTINGS: &str = "fade_ms = 0\nblur_radius = 4\nunredirect_fullscreen = true\n";

/// A blue window over the whole screen.
fn cover(desktop: &Desktop) -> Result<Window> {
    desktop.window(area(0, 0, 320, 240), 0x0000_00ff)
}

#[test]
fn a_window_that_covers_the_screen_suspends_compositing_with_present() -> Result<()> {
    a_window_that_covers_the_screen_suspends_compositing(SETTINGS)
}

#[test]
fn a_window_that_covers_the_screen_suspends_compositing_with_xrender() -> Result<()> {
    a_window_that_covers_the_screen_suspends_compositing(&format!("{SETTINGS}vsync = false\n"))
}

fn a_window_that_covers_the_screen_suspends_compositing(settings: &str) -> Result<()> {
    let mut desktop = Desktop::new(settings)?;
    let small = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(small)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    assert!(!desktop.suspended()?);
    // Given a window that covers the screen, mapped above the others, compositing stops:
    // the overlay leaves the screen and the window shows what it draws at once.
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_suspended(true)?;
    assert_eq!(desktop.screen_pixel((50, 50))?, BLUE);
    desktop.fill(full, area(40, 40, 30, 30), 0x0000_ff00)?;
    assert_eq!(desktop.screen_pixel((50, 50))?, GREEN);
    // When it goes, compositing resumes with the frame a new renderer would paint.
    desktop.conn.unmap_window(full)?.check()?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.until_pixel((200, 200), |p| p == BACKGROUND)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(
        &shown,
        &full_repaint(&desktop, settings)?,
        "after the cover",
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn what_windows_drew_while_suspended_shows_when_compositing_resumes() -> Result<()> {
    let mut desktop = Desktop::new(SETTINGS)?;
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_suspended(true)?;
    desktop.fill(full, area(100, 100, 50, 50), 0x0000_ff00)?;
    // Given a window mapped above the cover, compositing resumes, and the cover shows what
    // it drew meanwhile: its capture from before the suspension would lack the green square.
    let above = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(above)?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.until_pixel((120, 120), |p| p == GREEN)?;
    assert_eq!(desktop.pixel((200, 200))?, BLUE);
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop, SETTINGS)?, "a window above");
    // When the upper window goes, the cover hides everything again.
    desktop.conn.destroy_window(above)?.check()?;
    desktop.until_suspended(true)?;
    assert_eq!(desktop.screen_pixel((50, 50))?, BLUE);
    // The cover destroyed while suspended leaves the background, with no old image of it.
    desktop.conn.destroy_window(full)?.check()?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((120, 120), |p| p == BACKGROUND)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(
        &shown,
        &full_repaint(&desktop, SETTINGS)?,
        "the cover destroyed",
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn every_change_that_uncovers_the_screen_resumes_compositing() -> Result<()> {
    let mut desktop = Desktop::new(SETTINGS)?;
    let beneath = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(beneath)?;
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_suspended(true)?;
    let configure = |window, aux: &ConfigureWindowAux| -> Result<()> {
        desktop.conn.configure_window(window, aux)?.check()?;
        Ok(())
    };
    // Narrower than the screen, it shows the background beside it; restored, it covers again.
    configure(full, &ConfigureWindowAux::new().width(300))?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((310, 100), |p| p == BACKGROUND)?;
    assert_eq!(desktop.pixel((50, 50))?, BLUE);
    configure(full, &ConfigureWindowAux::new().width(320))?;
    desktop.until_suspended(true)?;
    // Moved, it uncovers a strip.
    configure(full, &ConfigureWindowAux::new().x(10))?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((5, 100), |p| p == BACKGROUND)?;
    configure(full, &ConfigureWindowAux::new().x(0))?;
    desktop.until_suspended(true)?;
    // Translucent, it shows the window beneath it.
    desktop.opacity(full, 0x8000_0000)?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((50, 50), |[r, _, b]| r > 100 && b > 100)?;
    desktop.opacity(full, 0xffff_ffff)?;
    desktop.until_suspended(true)?;
    // Shaped, it shows what lies around its shape.
    let half = [area(0, 0, 160, 240)];
    desktop
        .conn
        .shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            full,
            0,
            0,
            &half,
        )?
        .check()?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((200, 200), |p| p == BACKGROUND)?;
    desktop
        .conn
        .shape_mask(SO::SET, SK::BOUNDING, full, 0, 0, NONE)?
        .check()?;
    desktop.until_suspended(true)?;
    // Another window raised above it shows; lowered again, it is hidden.
    configure(
        beneath,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(
        &shown,
        &full_repaint(&desktop, SETTINGS)?,
        "a window raised",
    );
    configure(
        beneath,
        &ConfigureWindowAux::new().stack_mode(StackMode::BELOW),
    )?;
    desktop.until_suspended(true)?;
    // The setting reloaded off resumes compositing, and on suspends it.
    desktop.reload("fade_ms = 0\nblur_radius = 4\n")?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((50, 50), |p| p == BLUE)?;
    desktop.reload(SETTINGS)?;
    desktop.until_suspended(true)?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn windows_that_show_what_is_beneath_them_stay_composited() -> Result<()> {
    let mut desktop = Desktop::new(SETTINGS)?;
    let beneath = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(beneath)?;
    desktop.until_pixel((50, 50), |p| p == RED)?;
    // A window with an alpha channel over the whole screen is not a cover.
    let glass = desktop.argb_window(area(0, 0, 320, 240))?;
    desktop.map(glass)?;
    desktop.wait_vblanks(4)?;
    assert!(!desktop.suspended()?);
    assert_eq!(desktop.pixel((50, 50))?, RED);
    desktop.conn.destroy_window(glass)?.check()?;
    // Neither is one a pixel short of the screen.
    let short = desktop.window(area(0, 0, 320, 239), 0x0000_00ff)?;
    desktop.map(short)?;
    desktop.until_pixel((50, 50), |p| p == BLUE)?;
    desktop.wait_vblanks(4)?;
    assert!(!desktop.suspended()?);
    assert_eq!(desktop.pixel((100, 239))?, BACKGROUND);
    desktop.conn.destroy_window(short)?.check()?;
    // A window that receives no drawing, mapped above a cover, changes nothing on screen.
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_suspended(true)?;
    // Watch the root's children: the overlay coming back would report a map.
    let watch = ChangeWindowAttributesAux::new()
        .event_mask(EventMask::STRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_NOTIFY);
    desktop
        .conn
        .change_window_attributes(desktop.root, &watch)?
        .check()?;
    let unseen = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            0,
            unseen,
            desktop.root,
            0,
            0,
            320,
            240,
            0,
            WindowClass::INPUT_ONLY,
            0,
            &CreateWindowAux::new(),
        )?
        .check()?;
    desktop.map(unseen)?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(desktop.suspended()?);
    while let Some(event) = desktop.conn.poll_for_event()? {
        let resumed = matches!(event, Event::MapNotify(map) if map.window == desktop.overlay);
        assert!(
            !resumed,
            "compositing resumed for a window that shows nothing"
        );
    }
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn compositing_waits_for_a_cover_to_fade_in_and_never_stops_unasked() -> Result<()> {
    // Given a fade, the cover is translucent until it ends, and compositing stops only then.
    let desktop = Desktop::new("fade_ms = 300\nblur_radius = 0\nunredirect_fullscreen = true\n")?;
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_pixel((50, 50), |[_, _, b]| (60..200).contains(&b))?;
    assert!(!desktop.suspended()?);
    desktop.until_suspended(true)?;
    assert_eq!(desktop.screen_pixel((50, 50))?, BLUE);
    // Without the setting, a cover is composited like any window.
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\n")?;
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_pixel((50, 50), |p| p == BLUE)?;
    desktop.wait_vblanks(6)?;
    assert!(!desktop.suspended()?);
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn a_cover_closed_while_suspended_goes_without_its_old_image() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 400\nblur_radius = 0\nunredirect_fullscreen = true\n")?;
    let full = cover(&desktop)?;
    desktop.map(full)?;
    desktop.until_suspended(true)?;
    desktop.fill(full, area(0, 0, 320, 240), 0x0000_ff00)?;
    // Given a cover destroyed while compositing is suspended, Compust holds only its image
    // from before the suspension, still blue. Fading that out would show it for 400 ms; the
    // frames that follow show no blue at all, and then the background.
    desktop.conn.destroy_window(full)?.check()?;
    desktop.until_suspended(false)?;
    for _ in 0..6 {
        let [r, g, b] = desktop.pixel((160, 120))?;
        assert!(b < r.saturating_add(40), "the old image shows: {r} {g} {b}");
        desktop.wait_vblanks(1)?;
    }
    desktop.until_pixel((160, 120), |p| p == BACKGROUND)?;
    Ok(())
}

#[test]
fn suspending_and_resuming_leaks_nothing() -> Result<()> {
    let desktop = Desktop::new(SETTINGS)?;
    let beneath = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(beneath)?;
    let full = cover(&desktop)?;
    let round = || -> Result<()> {
        desktop.map(full)?;
        desktop.until_suspended(true)?;
        desktop.conn.unmap_window(full)?.check()?;
        desktop.until_suspended(false)?;
        desktop.until_pixel((50, 50), |p| p == RED)?;
        desktop.until_pixel((200, 200), |p| p == BACKGROUND)?;
        desktop.wait_vblanks(2)
    };
    round()?;
    let baseline = desktop.resources()?;
    for cycle in 0..4 {
        round()?;
        assert_eq!(desktop.resources()?, baseline, "cycle {cycle}");
    }
    Ok(())
}
