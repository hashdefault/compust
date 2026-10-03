use crate::{
    rect,
    support::{
        Desktop,
        monitors::Monitor,
        presentation::{Fault, Presentation},
    },
};
use anyhow::Result;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _};

#[test]
fn falls_back_to_xrender_after_present_rejection() -> Result<()> {
    let (mut desktop, mut presentation) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", Presentation::start)?;
    let window = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |pixel| pixel == [0, 0, 255])?;
    presentation.reject_next(Fault::Depth)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    let rejected = presentation.rejected()?;
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
    presentation.reject_next(fault)?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(180))?
        .check()?;
    presentation.rejected()?;
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
