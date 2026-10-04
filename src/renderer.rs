use crate::{
    config::Config,
    picture::{Format, Picture, Size},
    region::{Rect, Region},
    session::Session,
};
use anyhow::{Context, Result};
use std::{rc::Rc, time::Instant};
use x11rb::rust_connection::RustConnection;
mod blur;
mod damage;
mod paint;
mod present;
mod wallpaper;
pub(crate) use damage::Source;
use x11rb::{
    connection::Connection,
    protocol::{
        present::{ConnectionExt as _, EventMask},
        render::{ConnectionExt as _, QueryPictFormatsReply, Repeat},
        xfixes::ConnectionExt as _,
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
    output: Picture,
    alpha: Picture,
    wallpaper: Option<Picture>,
    pub(crate) present: bool,
    pub(crate) idle: bool,
    pub(crate) complete: bool,
    pub(crate) serial: u32,
    submission: Option<u16>,
    /// When the outstanding Present submission was made; `None` once it is abandoned.
    pub(crate) submitted: Option<Instant>,
    /// Blur pyramid, each level half the size of the previous one; empty without blur.
    levels: Vec<Picture>,
    /// The `blur_radius` and `vsync` settings the buffers and presentation were made for.
    built_for: (u8, bool),
    /// Changes to repaint beyond what the scene's own changes show.
    pending: Vec<(Source, Region)>,
    /// The surfaces the last frame showed, bottom to top.
    shown: Vec<damage::Shown>,
    /// Blurred backdrops kept from earlier frames, one per blurred surface.
    backdrops: Vec<blur::Backdrop>,
    /// The root's depth and picture format, for buffers.
    layout: Format,
    /// `XFixes` region naming the area each Present submission updates.
    update: u32,
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
        back.repeat(Repeat::PAD)?;
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
        let radius = if session.capabilities.bilinear {
            config.blur_radius
        } else {
            0
        };
        if radius != config.blur_radius {
            tracing::warn!("server lacks bilinear filtering; blur disabled");
        }
        let levels = blur::pyramid(
            conn,
            session.screen.root,
            (size, layout),
            blur::depth(radius),
        )?;
        let update = conn.generate_id()?;
        conn.xfixes_create_region(update, &[])?.check()?;
        let mut renderer = Self {
            conn: Rc::clone(conn),
            overlay: session.overlay,
            event_id,
            formats,
            size,
            back,
            output,
            alpha,
            wallpaper: None,
            present,
            idle: true,
            complete: true,
            serial: 0,
            submission: None,
            submitted: None,
            levels,
            built_for: (config.blur_radius, config.vsync),
            pending: Vec::new(),
            shown: Vec::new(),
            backdrops: Vec::new(),
            layout,
            update,
        };
        renderer.invalidate();
        renderer.refresh_wallpaper(session)?;
        Ok(renderer)
    }

    /// Whether a reloaded configuration can keep this renderer.
    pub(crate) fn fits(&self, config: &Config) -> bool {
        self.built_for == (config.blur_radius, config.vsync)
    }

    /// Repaint `rect` of `source` in the next frame, besides what changed in the scene.
    pub(crate) fn damage(&mut self, source: Source, rect: Rect) {
        if let Some((_, region)) = self.pending.iter_mut().find(|(kept, _)| *kept == source) {
            region.add(rect);
        } else {
            let mut region = Region::default();
            region.add(rect);
            self.pending.push((source, region));
        }
    }

    /// Repaint the whole screen in the next frame, blurring every backdrop again.
    pub(crate) fn invalidate(&mut self) {
        self.damage(Source::Background, self.screen());
    }

    fn screen(&self) -> Rect {
        Rect::new(0, 0, self.size.width, self.size.height)
    }
}
