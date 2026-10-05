mod client;

use crate::{
    animation::Fade,
    atoms::{Atoms, cardinal},
    config::Config,
    picture::{Picture, Size},
    region::Rect,
    rules::{self, Identity, Overrides, WindowType},
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
        xproto::{
            Atom, AtomEnum, ConnectionExt, GetGeometryReply, GetPropertyReply, MapState, Rectangle,
            Window, WindowClass,
        },
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
    /// What rules match on, from the client window.
    identity: Identity,
    /// The window `identity` came from.
    identified: Window,
    /// Whether the window manager leaves the window alone, which decides the type of a window
    /// that names none.
    override_redirect: bool,
    /// What the rules give this surface.
    pub(crate) overrides: Overrides,
    conn: Rc<RustConnection>,
}

pub(crate) struct Capture<'a> {
    pub(crate) conn: &'a Rc<RustConnection>,
    pub(crate) formats: &'a QueryPictFormatsReply,
    pub(crate) atoms: &'a Atoms,
    pub(crate) config: &'a Config,
    /// The window the window manager reports as active; see `atoms::active_window`.
    pub(crate) active: Option<Window>,
    /// The surface a resize replaces, whose client's identity it keeps.
    pub(crate) replaces: Option<&'a Surface>,
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
        let client = client_tree.client;
        let (identified, mut identity) = match context.replaces {
            Some(old) if !new_client(old.identified, client, window) => {
                (old.identified, old.identity.clone())
            }
            _ => (
                client,
                identity(conn, client, context.atoms, attr.override_redirect)?.unwrap_or_default(),
            ),
        };
        identity.focused = focused(context.active, identified);
        let overrides = rules::overrides(&context.config.rules, &identity);
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
            fade: Fade::opening(Instant::now(), fade_duration(context.config, overrides)),
            mapped: true,
            identity,
            identified,
            override_redirect: attr.override_redirect,
            overrides,
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

    pub(crate) fn refresh_client(
        &mut self,
        atoms: &Atoms,
        config: &Config,
        active: Option<Window>,
    ) -> Result<()> {
        self.client_tree = ClientTree::discover(&self.conn, self.window, atoms.wm_state)?;
        let client = self.client_tree.client;
        if new_client(self.identified, client, self.window) {
            self.identified = client;
            self.identity.focused = focused(active, client);
            self.refresh_identity(atoms, config)?;
        }
        self.refresh_opacity(atoms)
    }

    /// Follow the active window to `active`; reports whether the rules now give the surface
    /// other settings.
    pub(crate) fn focus(&mut self, active: Option<Window>, config: &Config) -> bool {
        let now = focused(active, self.identified);
        std::mem::replace(&mut self.identity.focused, now) != now && self.apply(config)
    }

    /// Whether the surface casts a shadow where shadows are on: as its rule says, or else as
    /// its client's type and decorations suggest. A shaped window casts none, because its
    /// shadow would be that of its bounding rectangle.
    pub(crate) fn casts_shadow(&self) -> bool {
        self.overrides.shadow.unwrap_or(self.identity.decorated) && self.rectangular()
    }

    /// The radius of the surface's rounded corners before its size limits it: as its rule
    /// says, or else the global one where its client's type and decorations suggest. A shaped
    /// window keeps its own shape, and a fullscreen one square corners.
    pub(crate) fn corner_radius(&self, config: &Config) -> u8 {
        if !self.rectangular() {
            return 0;
        }
        rules::corner_radius(config.corner_radius, self.overrides, &self.identity)
    }

    /// Whether the surface's shape is its whole rectangle, border included.
    pub(crate) fn rectangular(&self) -> bool {
        matches!(
            self.shape.as_slice(),
            [only] if (only.x, only.y) == (0, 0)
                && (only.width, only.height) == (self.size.width, self.size.height)
        )
    }

    /// Whether the identity comes from `window`.
    pub(crate) fn identified_by(&self, window: Window) -> bool {
        self.identified == window
    }

    /// Read the identity again from the window it came from, unless that window is gone; reports
    /// whether the surface now shows differently: the rules give it other settings, or it
    /// gains or loses its decorations or goes in or out of fullscreen.
    pub(crate) fn refresh_identity(&mut self, atoms: &Atoms, config: &Config) -> Result<bool> {
        let before = (self.identity.decorated, self.identity.fullscreen);
        if let Some(identity) =
            identity(&self.conn, self.identified, atoms, self.override_redirect)?
        {
            // Focus comes from the root, not from the client's properties.
            self.identity = Identity {
                focused: self.identity.focused,
                ..identity
            };
        }
        let ruled = self.apply(config);
        Ok(ruled || (self.identity.decorated, self.identity.fullscreen) != before)
    }

    /// Resolve the rules of `config` for this surface again, as after a reload; reports
    /// whether they now give it other settings.
    pub(crate) fn apply(&mut self, config: &Config) -> bool {
        let overrides = rules::overrides(&config.rules, &self.identity);
        std::mem::replace(&mut self.overrides, overrides) != overrides
    }

    /// How long this surface's fades take: its rule's `fade_ms`, or else the global one.
    pub(crate) fn fade_duration(&self, config: &Config) -> Duration {
        fade_duration(config, self.overrides)
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

    pub(crate) fn close(&mut self, now: Instant, config: &Config) {
        if self.mapped {
            self.fade.close(now, self.fade_duration(config));
            self.mapped = false;
        }
    }
}

fn fade_duration(config: &Config, overrides: Overrides) -> Duration {
    overrides.fade_ms.map_or_else(
        || config.fade_duration(),
        |ms| Duration::from_millis(u64::from(ms)),
    )
}

/// Whether `client` is the active window. Under a window manager that reports none, every
/// window counts, so that rules for unfocused windows change nothing.
fn focused(active: Option<Window>, client: Window) -> bool {
    active.is_none_or(|active| active == client)
}

/// Whether `client`, found in `frame`, is not the window an identity came from. A frame that
/// loses its client, as when the client closes, keeps the client's identity while it fades.
fn new_client(identified: Window, client: Window, frame: Window) -> bool {
    client != identified && client != frame
}

/// The identity of `client`, from its class, type, title, frame extents, and state, read in one
/// round trip; `None` when the client is gone.
fn identity(
    conn: &RustConnection,
    client: Window,
    atoms: &Atoms,
    override_redirect: bool,
) -> Result<Option<Identity>> {
    let requests = [
        (AtomEnum::WM_CLASS.into(), AtomEnum::STRING.into(), 256),
        (atoms.window_type, AtomEnum::ATOM.into(), 32),
        (AtomEnum::WM_TRANSIENT_FOR.into(), AtomEnum::ANY.into(), 0),
        (atoms.net_wm_name, atoms.utf8_string, 1024),
        (AtomEnum::WM_NAME.into(), AtomEnum::ANY.into(), 1024),
        (atoms.frame_extents, AtomEnum::CARDINAL.into(), 4),
        (atoms.net_wm_state, AtomEnum::ATOM.into(), 32),
    ];
    let cookies = requests
        .map(|(property, kind, words)| conn.get_property(false, client, property, kind, 0, words));
    // Every reply is read before any error returns: the error of a reply left unread would
    // arrive later as an event.
    let replies = cookies.map(|cookie| -> Result<GetPropertyReply> { Ok(cookie?.reply()?) });
    let [class, types, transient, utf8, legacy, extents, state] = match replies {
        [
            Ok(class),
            Ok(types),
            Ok(transient),
            Ok(utf8),
            Ok(legacy),
            Ok(extents),
            Ok(state),
        ] => [class, types, transient, utf8, legacy, extents, state],
        replies => {
            return match replies.into_iter().find_map(Result::err) {
                Some(error) if !window_gone(&error) => Err(error),
                _ => Ok(None),
            };
        }
    };
    let text = |reply: &GetPropertyReply, kind: u32| {
        reply.type_ == kind && reply.format == 8 && reply.bytes_after == 0
    };
    let class = text(&class, AtomEnum::STRING.into())
        .then(|| rules::class(&class.value))
        .flatten();
    let named = (types.type_ == u32::from(AtomEnum::ATOM) && types.bytes_after == 0)
        .then(|| types.value32())
        .flatten()
        .and_then(|mut values| {
            values.find_map(|atom| {
                atoms
                    .window_types
                    .iter()
                    .find_map(|&(known, kind)| (known == atom).then_some(kind))
            })
        });
    let transient = transient.type_ != u32::from(AtomEnum::NONE);
    let window_type = named.unwrap_or(if transient && !override_redirect {
        WindowType::Dialog
    } else {
        WindowType::Normal
    });
    let name = if text(&utf8, atoms.utf8_string) {
        String::from_utf8(utf8.value).ok()
    } else if text(&legacy, AtomEnum::STRING.into()) {
        Some(rules::latin1(&legacy.value))
    } else if text(&legacy, atoms.utf8_string) {
        String::from_utf8(legacy.value).ok()
    } else {
        None
    };
    // A client that keeps margins around its window draws its own shadow and corners in them.
    // A window that names no type is decorated only when the window manager handles it: bars,
    // menus, and tooltips of older toolkits are override-redirect windows without a type.
    let shades_itself = extents.type_ == u32::from(AtomEnum::CARDINAL)
        && extents.bytes_after == 0
        && extents
            .value32()
            .is_some_and(|mut margins| margins.any(|margin| margin > 0));
    let decorated = !shades_itself && named.map_or(!override_redirect, WindowType::decorated);
    Ok(Some(Identity {
        class,
        window_type,
        name,
        decorated,
        fullscreen: lists(&state, atoms.fullscreen),
        focused: false,
    }))
}

/// Whether `reply`, read as a list of atoms, holds `atom`. A property of another type or
/// format, or longer than the request read, holds none.
fn lists(reply: &GetPropertyReply, atom: Atom) -> bool {
    reply.type_ == u32::from(AtomEnum::ATOM)
        && reply.bytes_after == 0
        && reply
            .value32()
            .is_some_and(|mut atoms| atoms.any(|listed| listed == atom))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn property(kind: AtomEnum, format: u8, values: &[u32], bytes_after: u32) -> GetPropertyReply {
        GetPropertyReply {
            format,
            sequence: 0,
            length: 0,
            type_: kind.into(),
            bytes_after,
            value_len: u32::try_from(values.len()).unwrap(),
            value: values
                .iter()
                .flat_map(|value| value.to_ne_bytes())
                .collect(),
        }
    }

    #[test]
    fn states_come_only_from_a_whole_list_of_atoms() {
        // Given `_NET_WM_STATE` replies, a state counts only when a valid list holds it.
        assert!(lists(&property(AtomEnum::ATOM, 32, &[7, 9], 0), 9));
        assert!(!lists(&property(AtomEnum::ATOM, 32, &[7], 0), 9));
        assert!(!lists(&property(AtomEnum::ATOM, 32, &[], 0), 9));
        assert!(!lists(&property(AtomEnum::NONE, 0, &[], 0), 9));
        // Of another type, another format, or cut short, the property lists nothing.
        assert!(!lists(&property(AtomEnum::CARDINAL, 32, &[9], 0), 9));
        assert!(!lists(&property(AtomEnum::ATOM, 8, &[9], 0), 9));
        assert!(!lists(&property(AtomEnum::ATOM, 32, &[9], 4), 9));
    }
}
