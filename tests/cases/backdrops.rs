use crate::{
    rect,
    regions::{BACKGROUND, area, assert_same, confined, full_repaint, striped},
    support::{Desktop, presentation::Presentation},
};
use anyhow::{Context, Result};
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _, StackMode};

const BLUE: [u8; 3] = [0, 0, 255];
const GREEN: [u8; 3] = [0, 255, 0];

#[test]
fn kept_backdrops_follow_every_change_with_present() -> Result<()> {
    kept_backdrops_follow_every_change(true)
}

#[test]
fn kept_backdrops_follow_every_change_with_xrender() -> Result<()> {
    kept_backdrops_follow_every_change(false)
}

/// A blurred window reuses its backdrop for changes in or above it, and blurs again for each
/// kind of change beneath it. Every frame must match one that blurs everything again.
fn kept_backdrops_follow_every_change(vsync: bool) -> Result<()> {
    let settings = format!("fade_ms = 0\nblur_radius = 4\nvsync = {vsync}");
    let (mut desktop, mut presentation) = Desktop::with_display(&settings, Presentation::start)?;
    let background = striped(&desktop, 320)?;
    let under = desktop.window(area(60, 100, 40, 40), 0x0000_ff00)?;
    let blurred = desktop.window(rect(100, 60), 0x0000_0080)?;
    desktop.opacity(blurred, 0x8000_0000)?;
    let over = desktop.window(area(180, 140, 30, 30), 0x0000_00ff)?;
    for window in [under, blurred, over] {
        desktop.map(window)?;
    }
    desktop.until_pixel((195, 155), |p| p == BLUE)?;
    desktop.wait_vblanks(2)?;
    let step =
        |case: &str, within: Option<(i16, i16, u16, u16)>, change: &dyn Fn() -> Result<()>| {
            let before = presentation.updates()?.len();
            change()?;
            desktop.wait_vblanks(4)?;
            let shown = desktop.image()?;
            let updates = presentation.updates()?;
            let since = updates.get(before..).context("missing updates")?;
            if let Some(bounds) = within {
                assert!(confined(since, &[bounds]), "{case} repainted {since:?}");
            }
            assert_same(&shown, &full_repaint(&desktop, &settings)?, case);
            anyhow::Ok(())
        };

    // Changes in or above the blurred window reuse its backdrop.
    step("inside the blurred window", Some((110, 70, 4, 4)), &|| {
        desktop.fill(blurred, area(10, 10, 4, 4), 0x00ff_0000)
    })?;
    step("above it", Some((182, 142, 6, 6)), &|| {
        desktop.fill(over, area(2, 2, 6, 6), 0x0000_ff00)
    })?;
    step("its opacity", Some((100, 60, 100, 100)), &|| {
        desktop.opacity(blurred, 0xc000_0000)
    })?;

    // Changes beneath it, anywhere in its footprint, blur it again.
    step("the scene beneath it", None, &|| {
        desktop.fill(background, area(150, 100, 4, 4), 0x00ff_0000)
    })?;
    step("the scene beside it", None, &|| {
        desktop.fill(background, area(94, 100, 3, 3), 0x00ff_0000)
    })?;
    let configure = |window, aux: &ConfigureWindowAux| -> Result<()> {
        desktop.conn.configure_window(window, aux)?.check()?;
        Ok(())
    };
    step("a window beneath moved", None, &|| {
        configure(under, &ConfigureWindowAux::new().x(70))
    })?;
    step("a window beneath raised over it", None, &|| {
        configure(
            under,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )
    })?;
    step("a window above lowered beneath it", None, &|| {
        configure(
            over,
            &ConfigureWindowAux::new()
                .sibling(blurred)
                .stack_mode(StackMode::BELOW),
        )
    })?;
    step("a window beneath unmapped", None, &|| {
        desktop.conn.unmap_window(over)?.check()?;
        Ok(())
    })?;
    step("the blurred window moved", None, &|| {
        configure(blurred, &ConfigureWindowAux::new().x(120))
    })?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    presentation.finish()
}

#[test]
fn fading_blurred_windows_blur_once() -> Result<()> {
    let (desktop, mut presentation) =
        Desktop::with_display("fade_ms = 300\nblur_radius = 4", Presentation::start)?;
    let still = desktop.window(area(240, 20, 60, 60), 0x00ff_0000)?;
    desktop.map(still)?;
    desktop.until_pixel((250, 30), |p| p == [255, 0, 0])?;
    desktop.wait_vblanks(2)?;
    let before = presentation.updates()?.len();

    let fading = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.opacity(fading, 0x8000_0000)?;
    desktop.map(fading)?;
    desktop.until_pixel((50, 50), |[_, _, b]| b >= 140)?;
    desktop.conn.unmap_window(fading)?.check()?;
    desktop.until_pixel((50, 50), |p| p == BACKGROUND)?;
    desktop.wait_vblanks(2)?;

    // Only the first frame blurs; every later one reuses the backdrop within the window.
    let updates = presentation.updates()?;
    let fades = updates.get(before..).context("missing fade frames")?;
    let beyond: Vec<_> = fades
        .iter()
        .filter(|update| !confined(std::slice::from_ref(update), &[(20, 20, 100, 100)]))
        .collect();
    assert!(fades.len() > 4, "two fades took {} frames", fades.len());
    assert_eq!(beyond.len(), 1, "frames beyond the window: {beyond:?}");
    presentation.finish()
}

#[test]
fn stacked_blurs_keep_the_upper_backdrop_when_the_lower_blurs_again() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4";
    let mut desktop = Desktop::new(settings)?;
    let background = striped(&desktop, 320)?;
    let lower = desktop.window(rect(100, 60), 0x0000_0080)?;
    let upper = desktop.window(rect(150, 110), 0x0080_0000)?;
    for window in [lower, upper] {
        desktop.opacity(window, 0x8000_0000)?;
        desktop.map(window)?;
    }
    desktop.until_pixel((240, 200), |[r, g, _]| r > g.saturating_add(30))?;
    desktop.wait_vblanks(2)?;

    // The upper window's footprint starts at (142, 100), so this change lies beneath the
    // lower window alone. The lower window blurs again across its footprint, which reaches
    // into the upper window, and the upper window repaints there from its kept backdrop.
    desktop.fill(background, area(130, 104, 12, 50), 0x00ff_0000)?;
    desktop.wait_vblanks(4)?;
    let shown = desktop.image()?;
    assert_same(
        &shown,
        &full_repaint(&desktop, settings)?,
        "beneath a lower blur",
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn blur_shows_as_strongly_as_the_window_covers_it() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 4")?;
    striped(&desktop, 320)?;
    // Like a browser menu: an opaque body in a transparent margin, here with a
    // half-transparent band along the bottom.
    let menu = desktop.argb_window(area(100, 60, 120, 100))?;
    desktop.map(menu)?;
    desktop.fill(menu, area(20, 20, 80, 60), 0xff00_ff00)?;
    desktop.fill(menu, area(0, 90, 120, 10), 0x8000_0000)?;
    desktop.until_pixel((160, 110), |p| p == GREEN)?;
    desktop.wait_vblanks(2)?;
    // The transparent margin leaves the stripes sharp: no halo of blur around the body.
    assert_eq!(desktop.pixel((104, 70))?, [0, 0, 0]);
    assert_eq!(desktop.pixel((105, 70))?, [255, 255, 255]);
    // The half-transparent band shows half the blur beneath its half-black tint.
    let gray = |[r, g, b]: [u8; 3]| r == g && g == b;
    let dark = desktop.pixel((104, 155))?;
    let light = desktop.pixel((105, 155))?;
    assert!(gray(dark) && (25..=40).contains(&dark[0]), "{dark:?}");
    assert!(gray(light) && (88..=104).contains(&light[0]), "{light:?}");
    Ok(())
}
