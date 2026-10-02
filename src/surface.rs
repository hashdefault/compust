use crate::{
    animation::Fade,
    atoms::{Atoms, cardinal},
    config::Config,
    picture::{Picture, Size},
};
use anyhow::{Context, Result};
use std::{rc::Rc, time::Instant};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        composite::ConnectionExt as _,
        damage::{ConnectionExt as _, ReportLevel},
        render::QueryPictFormatsReply,
        shape::{ConnectionExt as _, SK},
        xproto::{
            Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, GetGeometryReply,
            MapState, Rectangle, Window, WindowClass,
        },
    },
    rust_connection::RustConnection,
};

pub(crate) struct Surface {
    pub(crate) window: Window,
    pub(crate) client: Window,
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
        let client = find_client(conn, window, context.atoms.wm_state)?.unwrap_or(window);
        for watched in [window, client] {
            conn.change_window_attributes(
                watched,
                &ChangeWindowAttributesAux::new()
                    .event_mask(EventMask::PROPERTY_CHANGE | EventMask::SUBSTRUCTURE_NOTIFY),
            )?
            .check()?;
        }
        conn.shape_select_input(window, true)?.check()?;
        let damage = conn.generate_id()?;
        conn.damage_create(damage, pixmap, ReportLevel::NON_EMPTY)?
            .check()?;
        let mut surface = Self {
            window,
            client,
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
        self.shape = self
            .conn
            .shape_get_rectangles(self.window, SK::BOUNDING)?
            .reply()?
            .rectangles;
        for rect in &mut self.shape {
            rect.x = rect
                .x
                .saturating_add(border)
                .saturating_add(self.geometry.x);
            rect.y = rect
                .y
                .saturating_add(border)
                .saturating_add(self.geometry.y);
        }
        Ok(())
    }

    pub(crate) fn refresh_opacity(&mut self, atoms: &Atoms) -> Result<()> {
        let frame = cardinal(&self.conn, self.window, atoms.opacity)?;
        let opacity = match frame {
            Some(value) => value,
            None if self.client != self.window => {
                cardinal(&self.conn, self.client, atoms.opacity)?.unwrap_or(u32::MAX)
            }
            None => u32::MAX,
        };
        self.opacity = u16::try_from(opacity >> 16)?;
        Ok(())
    }

    pub(crate) fn close(&mut self, now: Instant) {
        if self.mapped {
            self.fade.close(now);
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

fn find_client(conn: &RustConnection, window: Window, state: Atom) -> Result<Option<Window>> {
    let mut pending = vec![(window, 0_u8)];
    while let Some((candidate, depth)) = pending.pop() {
        let property = conn
            .get_property(false, candidate, state, AtomEnum::ANY, 0, 0)?
            .reply()?;
        if property.type_ != NONE {
            return Ok(Some(candidate));
        }
        if depth < 8 {
            pending.extend(
                conn.query_tree(candidate)?
                    .reply()?
                    .children
                    .into_iter()
                    .map(|child| (child, depth.saturating_add(1))),
            );
        }
    }
    Ok(None)
}
