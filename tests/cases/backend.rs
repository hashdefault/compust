use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _};

#[test]
fn the_gpu_backend_falls_back_to_xrender_without_dri3() -> Result<()> {
    // Xvfb has no DRI3, so the GPU renderer cannot open and XRender paints instead.
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 4\nbackend = \"gl\"")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;

    // Reloads that switch the backend replace the renderer and keep painting.
    for settings in [
        "fade_ms = 0\nblur_radius = 4",
        "fade_ms = 0\nblur_radius = 4\nbackend = \"gl\"",
    ] {
        desktop.reload(settings)?;
        desktop.wait_vblanks(4)?;
        let x = desktop.conn.get_geometry(window)?.reply()?.x + 40;
        desktop
            .conn
            .configure_window(window, &ConfigureWindowAux::new().x(i32::from(x)))?
            .check()?;
        let shown = (x + 20, 40);
        desktop.until_pixel(shown, |p| p == [255, 0, 0])?;
    }
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
