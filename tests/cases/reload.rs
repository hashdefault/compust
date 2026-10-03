use crate::{
    rect,
    support::{
        Desktop,
        presentation::{Fault, Presentation},
    },
};
use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::xproto::{ConfigureWindowAux, ConnectionExt as _, CreateGCAux, Rectangle},
};

/// A saturated channel at 50% global opacity over the default background.
fn half_red([r, g, b]: [u8; 3]) -> bool {
    (136..=142).contains(&r) && (11..=13).contains(&g) && (15..=17).contains(&b)
}

fn half_blue([r, g, b]: [u8; 3]) -> bool {
    (11..=13).contains(&r) && (11..=13).contains(&g) && (140..=146).contains(&b)
}

#[test]
fn reload_applies_opacity_without_restarting() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;

    desktop.reload("fade_ms = 0\nblur_radius = 0\nopacity = 50")?;

    desktop.until_pixel((40, 40), half_red)?;
    desktop.reload("fade_ms = 0\nblur_radius = 0")?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn reloaded_fade_duration_closes_windows_opened_before_it() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    // The opacity change shows that the reload was applied before the window closes.
    desktop.reload("fade_ms = 1000\nblur_radius = 0\nopacity = 50")?;
    desktop.until_pixel((40, 40), half_red)?;

    desktop.conn.unmap_window(window)?.check()?;

    desktop.until_pixel((40, 40), |[r, _, _]| (40..=125).contains(&r))?;
    desktop.screenshot("reload-fade")?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn rejected_reloads_keep_the_running_configuration() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\nopacity = 50")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), half_red)?;

    desktop.reload("fade_ms = 0\nblur_radius = 0\nopacity = 101")?;
    // A later window shows a paint after the reload; it must still use 50% opacity.
    let second = desktop.window(rect(160, 20), 0x0000_00ff)?;
    desktop.map(second)?;
    desktop.until_pixel((180, 40), half_blue)?;
    assert!(half_red(desktop.pixel((40, 40))?));

    desktop.reload("fade_ms = 0\nblur_radius = 0\nopacity = [")?;
    desktop.conn.unmap_window(second)?.check()?;
    desktop.until_pixel((180, 40), |p| p == [24, 24, 32])?;
    assert!(half_red(desktop.pixel((40, 40))?));

    desktop.reload_missing()?;
    desktop.map(second)?;
    desktop.until_pixel((180, 40), half_blue)?;
    assert!(half_red(desktop.pixel((40, 40))?));
    assert!(desktop.compositor.0.try_wait()?.is_none());

    desktop.reload("fade_ms = 0\nblur_radius = 0")?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.until_pixel((180, 40), |p| p == [0, 0, 255])?;
    Ok(())
}

#[test]
fn blur_reloads_replace_the_renderer_without_growth_with_present() -> Result<()> {
    repeated_blur_reload(true)
}

#[test]
fn blur_reloads_replace_the_renderer_without_growth_with_xrender() -> Result<()> {
    repeated_blur_reload(false)
}

/// Toggle blur behind a translucent window over one-pixel stripes. Each reload replaces the
/// renderer and its blur buffers; the sharp scenes must hold the same server resources.
fn repeated_blur_reload(vsync: bool) -> Result<()> {
    let sharp = format!("fade_ms = 0\nblur_radius = 0\nvsync = {vsync}");
    let blurred = format!("fade_ms = 0\nblur_radius = 4\nvsync = {vsync}");
    let mut desktop = Desktop::new(&sharp)?;
    let background = desktop.window(
        Rectangle {
            x: 0,
            y: 0,
            width: 320,
            height: 240,
        },
        0x00ff_ffff,
    )?;
    let gc = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_gc(gc, background, &CreateGCAux::new().foreground(0))?
        .check()?;
    desktop.map(background)?;
    let stripes: Vec<_> = (0_i16..320)
        .step_by(2)
        .map(|x| Rectangle {
            x,
            y: 0,
            width: 1,
            height: 240,
        })
        .collect();
    desktop
        .conn
        .poly_fill_rectangle(background, gc, &stripes)?
        .check()?;
    let front = desktop.window(rect(100, 60), 0)?;
    desktop.opacity(front, 0x8000_0000)?;
    desktop.map(front)?;
    // A white stripe behind the half-opaque black window, then the stripes' average.
    let is_sharp = |[r, g, b]: [u8; 3]| (120..=135).contains(&r) && r == g && g == b;
    let is_blurred = |[r, g, b]: [u8; 3]| (40..=90).contains(&r) && r == g && g == b;
    desktop.until_pixel((141, 100), is_sharp)?;
    let cycle = |desktop: &Desktop| -> Result<()> {
        desktop.reload(&blurred)?;
        desktop.until_pixel((141, 100), is_blurred)?;
        desktop.reload(&sharp)?;
        desktop.until_pixel((141, 100), is_sharp)?;
        Ok(())
    };
    for _ in 0..2 {
        cycle(&desktop)?;
    }
    let baseline = desktop.resources()?;

    for reload in 0..16 {
        cycle(&desktop)?;
        assert_eq!(
            desktop.resources()?,
            baseline,
            "blur reload {reload}, vsync={vsync}"
        );
    }
    eprintln!("blur reloads vsync={vsync}: {baseline:?}");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn reload_switches_between_present_and_xrender() -> Result<()> {
    let (mut desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    desktop.reload("fade_ms = 0\nblur_radius = 0\nopacity = 50\nvsync = false")?;
    desktop.until_pixel((40, 40), half_blue)?;
    let direct = presentation.submissions();
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    desktop.until_pixel((200, 40), half_blue)?;
    assert_eq!(
        presentation.submissions(),
        direct,
        "Present used after a reload disabled vsync"
    );

    desktop.reload("fade_ms = 0\nblur_radius = 0")?;
    desktop.until_pixel((200, 40), |p| p == [0, 0, 255])?;
    assert!(
        presentation.submissions() > direct,
        "Present not resumed after a reload enabled vsync"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    presentation.finish()
}

#[test]
fn reload_keeps_xrender_after_present_rejection() -> Result<()> {
    let (mut desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    presentation.inject(Fault::Depth)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    let rejected = presentation.injected()?;
    desktop.until_pixel((200, 40), |p| p == [0, 0, 255])?;

    // The file still enables vsync, and the blur change forces a new renderer.
    desktop.reload("fade_ms = 0\nblur_radius = 4\nopacity = 50")?;
    desktop.until_pixel((200, 40), half_blue)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(20))?
        .check()?;
    desktop.until_pixel((40, 40), half_blue)?;

    assert_eq!(
        presentation.submissions(),
        rejected,
        "Present retried after a reload"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    presentation.finish()
}
