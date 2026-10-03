use crate::{
    rect,
    support::{
        Desktop,
        monitors::Monitor,
        presentation::{Fault, Presentation},
    },
};
use anyhow::Result;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _, Window};

#[test]
fn falls_back_to_xrender_after_present_rejection() -> Result<()> {
    let (mut desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |pixel| pixel == [0, 0, 255])?;
    presentation.inject(Fault::Depth)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    let rejected = presentation.injected()?;
    desktop.until_pixel((200, 40), |pixel| pixel == [0, 0, 255])?;
    assert_eq!(desktop.pixel((40, 40))?, [24, 24, 32]);

    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(20))?
        .check()?;
    desktop.until_pixel((200, 40), |pixel| pixel == [24, 24, 32])?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
    let monitor = Monitor::new(&desktop)?;
    monitor.disable(&desktop)?;
    desktop.resize_root((240, 180))?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(120))?
        .check()?;
    desktop.until_pixel((140, 40), |pixel| pixel == [0, 0, 255])?;
    desktop.resize_root((320, 240))?;
    monitor.restore(&desktop)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(220))?
        .check()?;
    desktop.until_pixel((300, 40), |pixel| pixel == [0, 0, 255])?;
    assert_eq!(
        presentation.submissions(),
        rejected,
        "Present retried after fallback or root resize"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    desktop.screenshot("presentation-fallback")?;
    presentation.finish()
}

#[test]
fn replaces_unfinished_presentation_after_root_resize() -> Result<()> {
    let (mut desktop, mut presentation, window, stalled) = stalled_presentation()?;
    let monitor = Monitor::new(&desktop)?;
    monitor.disable(&desktop)?;
    desktop.resize_root((240, 180))?;
    move_into_view(&desktop, window, 120)?;
    desktop.resize_root((320, 240))?;
    monitor.restore(&desktop)?;
    move_into_view(&desktop, window, 220)?;
    finish_replacement(&mut desktop, &mut presentation, stalled)
}

#[test]
fn replaces_unfinished_presentation_after_output_toggle() -> Result<()> {
    let (mut desktop, mut presentation, window, stalled) = stalled_presentation()?;
    let monitor = Monitor::new(&desktop)?;
    monitor.disable(&desktop)?;
    move_into_view(&desktop, window, 120)?;
    monitor.restore(&desktop)?;
    move_into_view(&desktop, window, 220)?;
    finish_replacement(&mut desktop, &mut presentation, stalled)
}

#[test]
fn replaces_unfinished_presentation_after_timeout() -> Result<()> {
    let (mut desktop, mut presentation, window, stalled) = stalled_presentation()?;
    // No monitor change follows: the withheld frame itself must reach the screen.
    desktop.until_pixel((200, 40), |pixel| pixel == [0, 0, 255])?;
    assert_eq!(desktop.pixel((40, 40))?, [24, 24, 32]);
    move_into_view(&desktop, window, 120)?;
    move_into_view(&desktop, window, 220)?;
    finish_replacement(&mut desktop, &mut presentation, stalled)
}

/// Hold back a submission's completion and idle events, as a CRTC change did on hardware.
fn stalled_presentation() -> Result<(Desktop, Presentation, Window, u32)> {
    let (desktop, presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |pixel| pixel == [0, 0, 255])?;
    presentation.inject(Fault::Fence)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    let stalled = presentation.injected()?;
    Ok((desktop, presentation, window, stalled))
}

fn move_into_view(desktop: &Desktop, window: Window, x: i32) -> Result<()> {
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(x))?
        .check()?;
    let point = (i16::try_from(x)? + 20, 40);
    desktop.until_pixel(point, |pixel| pixel == [0, 0, 255])?;
    Ok(())
}

fn finish_replacement(
    desktop: &mut Desktop,
    presentation: &mut Presentation,
    stalled: u32,
) -> Result<()> {
    assert!(
        presentation.submissions() > stalled,
        "presentation did not continue through Present"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    presentation.finish()
}

#[test]
fn invalid_present_pixmap_remains_fatal() -> Result<()> {
    fatal(Fault::Pixmap)
}

#[test]
fn invalid_present_window_remains_fatal() -> Result<()> {
    fatal(Fault::Window)
}

fn fatal(fault: Fault) -> Result<()> {
    let (mut desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |pixel| pixel == [0, 0, 255])?;
    presentation.inject(fault)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    presentation.injected()?;
    assert!(
        desktop
            .until_pixel((200, 40), |pixel| pixel == [0, 0, 255])
            .is_err()
    );
    assert!(
        desktop
            .compositor
            .0
            .try_wait()?
            .is_some_and(|status| !status.success())
    );
    presentation.finish()
}
