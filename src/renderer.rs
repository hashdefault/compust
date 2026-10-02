use crate::{
    config::Config,
    picture::{Format, Picture, Size},
    session::Session,
};
use anyhow::{Context, Result};
use std::rc::Rc;
use x11rb::rust_connection::RustConnection;
mod blur;
mod paint;
mod present;
mod wallpaper;
use x11rb::{
    connection::Connection,
    protocol::{
        present::{ConnectionExt as _, EventMask},
        render::{ConnectionExt as _, QueryPictFormatsReply, Repeat},
        xproto::ConnectionExt as _,
    },
};

pub(crate) struct Renderer {
    conn: Rc<RustConnection>,
    overlay: u32,
    pub(crate) event_id: Option<u32>,
    pub(crate) formats: QueryPictFormatsReply,
    pub(crate) size: Size,
    pub(crate) back: Picture,
    scratch: Picture,
    output: Picture,
    alpha: Picture,
    wallpaper: Option<Picture>,
    pub(crate) present: bool,
    pub(crate) idle: bool,
    pub(crate) complete: bool,
    pub(crate) serial: u32,
    horizontal: Vec<i32>,
    vertical: Vec<i32>,
}

impl Renderer {
    pub(crate) fn new(session: &Session, config: &Config) -> Result<Self> {
        let conn = &session.conn;
        let formats = conn.render_query_pict_formats()?.reply()?;
        let format = formats
            .screens
            .get(session.screen_number)
            .context("missing render screen")?
            .depths
            .iter()
            .flat_map(|d| &d.visuals)
            .find(|v| v.visual == session.screen.root_visual)
            .context("missing root visual")?
            .format;
        let layout = Format {
            depth: session.screen.root_depth,
            id: format,
        };
        let geometry = conn.get_geometry(session.screen.root)?.reply()?;
        let size = Size {
            width: geometry.width,
            height: geometry.height,
        };
        let back = Picture::buffer(conn, session.screen.root, (size, layout))?;
        let scratch = Picture::buffer(conn, session.screen.root, (size, layout))?;
        back.repeat(Repeat::PAD)?;
        scratch.repeat(Repeat::PAD)?;
        let output = Picture::borrowed(conn, session.overlay, format)?;
        let a8 = formats
            .formats
            .iter()
            .find(|f| f.depth == 8 && f.direct.alpha_mask == 255)
            .context("missing A8 render format")?
            .id;
        let alpha = Picture::buffer(
            conn,
            session.screen.root,
            (
                Size {
                    width: 1,
                    height: 1,
                },
                Format { depth: 8, id: a8 },
            ),
        )?;
        alpha.repeat(Repeat::NORMAL)?;
        let present = config.vsync && session.capabilities.present;
        let event_id = if present {
            Some(conn.generate_id()?)
        } else {
            None
        };
        if let Some(id) = event_id {
            conn.present_select_input(
                id,
                session.overlay,
                EventMask::COMPLETE_NOTIFY | EventMask::IDLE_NOTIFY,
            )?
            .check()?;
        }
        let radius = if session.capabilities.convolution {
            config.blur_radius
        } else {
            0
        };
        if radius != config.blur_radius {
            tracing::warn!("server lacks convolution; blur disabled");
        }
        let (horizontal, vertical) = blur::kernel(radius);
        let mut renderer = Self {
            conn: Rc::clone(conn),
            overlay: session.overlay,
            event_id,
            formats,
            size,
            back,
            scratch,
            output,
            alpha,
            wallpaper: None,
            present,
            idle: true,
            complete: true,
            serial: 0,
            horizontal,
            vertical,
        };
        renderer.refresh_wallpaper(session)?;
        Ok(renderer)
    }
}
