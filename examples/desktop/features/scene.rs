use crate::surface::Surface;
use anyhow::Result;
use x11rb::{
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT,
    connection::Connection as _,
    protocol::xproto::{
        AtomEnum, ConnectionExt as _, CreateWindowAux, MapState, PropMode, Rectangle, WindowClass,
    },
    wrapper::ConnectionExt as _,
};

pub(super) const WHITE: u32 = 0x00ff_ffff;
pub(super) const RED: u32 = 0x00ff_0000;
pub(super) const BLUE: u32 = 0x0000_00ff;
pub(super) const GREEN: u32 = 0x0000_ff00;

pub(super) fn create(surface: &Surface, area: Rectangle, color: u32) -> Result<u32> {
    let window = surface.conn.generate_id()?;
    surface
        .conn
        .create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            surface.root,
            area.x,
            area.y,
            area.width,
            area.height,
            0,
            WindowClass::INPUT_OUTPUT,
            COPY_FROM_PARENT,
            &CreateWindowAux::new()
                .background_pixel(color)
                .override_redirect(1),
        )?
        .check()?;
    typed(surface, window, "_NET_WM_WINDOW_TYPE_NORMAL")?;
    Ok(window)
}

pub(super) fn typed(surface: &Surface, window: u32, name: &str) -> Result<()> {
    surface
        .conn
        .change_property32(
            PropMode::REPLACE,
            window,
            surface.atom("_NET_WM_WINDOW_TYPE")?,
            AtomEnum::ATOM,
            &[surface.atom(name)?],
        )?
        .check()?;
    Ok(())
}

pub(super) fn activate(surface: &Surface, window: u32) -> Result<()> {
    surface
        .conn
        .change_property32(
            PropMode::REPLACE,
            surface.root,
            surface.atom("_NET_ACTIVE_WINDOW")?,
            AtomEnum::WINDOW,
            &[window],
        )?
        .check()?;
    Ok(())
}

pub(super) fn suspended(surface: &Surface) -> Result<bool> {
    Ok(surface
        .conn
        .get_window_attributes(surface.overlay)?
        .reply()?
        .map_state
        == MapState::UNMAPPED)
}

pub(super) fn wait_state(surface: &Surface, state: bool) -> Result<()> {
    surface.until(
        if state {
            "fullscreen suspension"
        } else {
            "compositing resumed"
        },
        || Ok(suspended(surface)? == state),
    )
}

pub(super) fn near(pixel: u32, expected: u32) -> bool {
    [16, 8, 0]
        .into_iter()
        .all(|shift| ((pixel >> shift) & 255).abs_diff((expected >> shift) & 255) <= 2)
}
