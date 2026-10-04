mod client;

use crate::{
    animation::Fade,
    atoms::{Atoms, cardinal},
    config::Config,
    picture::{Picture, Size},
    region::Rect,
};
use anyhow::{Context, Result};
use client::{ClientTree, window_gone};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        composite::ConnectionExt as _,
        damage::{ConnectionExt as _, ReportLevel},
        render::QueryPictFormatsReply,
        shape::{ConnectionExt as _, SK},
        xproto::{ConnectionExt, GetGeometryReply, MapState, Rectangle, Window, WindowClass},
    },
    rust_connection::RustConnection,
};

pub(crate) struct Surface {
    pub(crate) window: Window,
    client_tree: ClientTree,
    pub(crate) picture: Picture,
    pub(crate) damage: u32,
    pub(crate) geometry: GetGeometryReply,
    pub(crate) size: Size,
    pub(crate) shape: Vec<Rectangle>,
    pub(crate) opacity: u16,
    pub(crate) has_alpha: bool,
    pub(crate) fade: Fade,
    pub(crate) mapped: bool,
    conn: Rc<RustConnection>,
}

pub(crate) struct Capture<'a> {
    pub(crate) conn: &'a Rc<RustConnection>,
    pub(crate) formats: &'a QueryPictFormatsReply,
    pub(crate) atoms: &'a Atoms,
    pub(crate) config: &'a Config,
}

impl Surface {
    pub(crate) fn capture(window: Window, context: &Capture<'_>) -> Result<Option<Self>> {
        let conn = context.conn;
        let attr = conn.get_window_attributes(window)?.reply()?;
        if attr.class == WindowClass::INPUT_ONLY || attr.map_state != MapState::VIEWABLE {
            return Ok(None);
        }
        let geometry = conn.get_geometry(window)?.reply()?;
        let format = context
            .formats
            .screens
            .iter()
            .flat_map(|s| &s.depths)
            .flat_map(|d| &d.visuals)
            .find(|v| v.visual == attr.visual)
            .context("window has no XRender visual")?
            .format;
        let has_alpha = context
            .formats
            .formats
            .iter()
            .find(|f| f.id == format)
            .is_some_and(|f| f.direct.alpha_mask != 0);
        let pixmap = conn.generate_id()?;
        conn.composite_name_window_pixmap(window, pixmap)?.check()?;
        let mut picture = match Picture::borrowed(conn, pixmap, format) {
            Ok(picture) => picture,
            Err(error) => {
                conn.free_pixmap(pixmap)?;
                return Err(error);
            }
        };
        picture.pixmap = Some(pixmap);
        let client_tree = ClientTree::discover(conn, window, context.atoms.wm_state)?;
        conn.shape_select_input(window, true)?.check()?;
        let damage = conn.generate_id()?;
        // Each report carries the extents of the damage since the last subtraction, which
        // locates it without fetching the region.
        conn.damage_create(damage, pixmap, ReportLevel::BOUNDING_BOX)?
            .check()?;
        let mut surface = Self {
            window,
            client_tree,
            picture,
            damage,
            size: Size {
                width: 1,
                height: 1,
            },
            geometry,
            shape: Vec::new(),
            opacity: u16::MAX,
            has_alpha,
            fade: Fade::opening(Instant::now(), context.config.fade_duration()),
            mapped: true,
            conn: Rc::clone(conn),
        };
        surface.refresh_shape()?;
        surface.refresh_opacity(context.atoms)?;
        Ok(Some(surface))
    }

    pub(crate) fn refresh_shape(&mut self) -> Result<()> {
        let border = self.geometry.border_width;
        self.size = Size {
            width: self
                .geometry
                .width
                .checked_add(border.saturating_mul(2))
                .context("window width exceeds X11 limits")?,
            height: self
                .geometry
                .height
                .checked_add(border.saturating_mul(2))
                .context("window height exceeds X11 limits")?,
        };
        let border = i16::try_from(border)?;
        self.shape = if border > 0
            && !self
                .conn
                .shape_query_extents(self.window)?
                .reply()?
                .bounding_shaped
        {
            // The default ShapeGetRectangles rectangle can omit the right/bottom border.
            vec![Rectangle {
                x: 0,
                y: 0,
                width: self.size.width,
                height: self.size.height,
            }]
        } else {
            let mut shape = self
                .conn
                .shape_get_rectangles(self.window, SK::BOUNDING)?
                .reply()?
                .rectangles;
            for rect in &mut shape {
                rect.x = rect.x.saturating_add(border);
                rect.y = rect.y.saturating_add(border);
            }
            shape
        };
        Ok(())
    }

    /// Where the surface lies on the root, border included.
    pub(crate) fn bounds(&self) -> Rect {
        Rect::new(
            i32::from(self.geometry.x),
            i32::from(self.geometry.y),
            self.size.width,
            self.size.height,
        )
    }

    /// The root area of `area`, given in the surface's pixmap.
    pub(crate) fn damaged(&self, area: Rectangle) -> Rect {
        Rect::at(area, (self.geometry.x, self.geometry.y))
    }

    pub(crate) fn watches(&self, window: Window) -> bool {
        self.client_tree.watched.contains(&window)
    }

    pub(crate) fn refresh_client(&mut self, atoms: &Atoms) -> Result<()> {
        self.client_tree = ClientTree::discover(&self.conn, self.window, atoms.wm_state)?;
        self.refresh_opacity(atoms)
    }

    pub(crate) fn refresh_opacity(&mut self, atoms: &Atoms) -> Result<()> {
        let frame = cardinal(&self.conn, self.window, atoms.opacity)?;
        let opacity = match frame {
            Some(value) => value,
            None if self.client_tree.client != self.window => {
                match cardinal(&self.conn, self.client_tree.client, atoms.opacity) {
                    Ok(value) => value.unwrap_or(u32::MAX),
                    Err(error) if window_gone(&error) => u32::MAX,
                    Err(error) => return Err(error),
                }
            }
            None => u32::MAX,
        };
        self.opacity = u16::try_from(opacity >> 16)?;
        Ok(())
    }

    pub(crate) fn close(&mut self, now: Instant, fade: Duration) {
        if self.mapped {
            self.fade.close(now, fade);
            self.mapped = false;
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if let Err(error) = self
            .conn
            .damage_destroy(self.damage)
            .map(x11rb::cookie::VoidCookie::ignore_error)
        {
            tracing::debug!(%error, "damage cleanup failed");
        }
    }
}
