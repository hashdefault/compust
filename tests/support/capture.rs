use anyhow::{Context, Result};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use x11rb::{
    connection::RequestConnection,
    protocol::{
        composite, damage, render, shape,
        xproto::{self, ConnectionExt, Window},
    },
    rust_connection::RustConnection,
    x11_utils::{RequestHeader, TryParse},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapturePoint {
    Attributes,
    Geometry,
    Pixmap,
    Picture,
    ClientEvents,
    ClientState,
    ClientTree,
    ShapeEvents,
    Damage,
    Shape,
    Opacity,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Resources {
    pub(crate) pixmap: Option<u32>,
    pub(crate) picture: Option<u32>,
    pub(crate) damage: Option<u32>,
}

pub(super) struct Race {
    pub(super) window: Window,
    pub(super) point: CapturePoint,
    resources: Resources,
}

impl Race {
    pub(super) fn new(window: Window, point: CapturePoint) -> Self {
        Self {
            window,
            point,
            resources: Resources::default(),
        }
    }
}

pub(super) struct Interceptor {
    conn: RustConnection,
    extensions: [u8; 4],
    state: u32,
    opacity: u32,
    commands: Receiver<Race>,
    completed: SyncSender<Resources>,
    active: Option<Race>,
}

impl Interceptor {
    pub(super) fn new(
        display: &str,
        commands: Receiver<Race>,
        completed: SyncSender<Resources>,
    ) -> Result<Self> {
        let (conn, _) = x11rb::connect(Some(display))?;
        let mut extensions = [0; 4];
        for (opcode, name) in extensions.iter_mut().zip([
            composite::X11_EXTENSION_NAME,
            render::X11_EXTENSION_NAME,
            shape::X11_EXTENSION_NAME,
            damage::X11_EXTENSION_NAME,
        ]) {
            *opcode = conn
                .extension_information(name)?
                .context("missing capture extension")?
                .major_opcode;
        }
        let state = conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;
        let opacity = conn
            .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
            .reply()?
            .atom;
        Ok(Self {
            conn,
            extensions,
            state,
            opacity,
            commands,
            completed,
            active: None,
        })
    }

    pub(super) fn before(&mut self, header: RequestHeader, body: &[u8]) -> Result<()> {
        match self.commands.try_recv() {
            Ok(race) => self.active = Some(race),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => (),
        }
        let Some(race) = &mut self.active else {
            return Ok(());
        };
        let [composite, render, shape, damage] = self.extensions;
        let point = match (header.major_opcode, header.minor_opcode) {
            (xproto::GET_WINDOW_ATTRIBUTES_REQUEST, _) => CapturePoint::Attributes,
            (xproto::GET_GEOMETRY_REQUEST, _) => CapturePoint::Geometry,
            (xproto::CHANGE_WINDOW_ATTRIBUTES_REQUEST, _) => CapturePoint::ClientEvents,
            (xproto::QUERY_TREE_REQUEST, _) => CapturePoint::ClientTree,
            (xproto::GET_PROPERTY_REQUEST, _) => {
                let (_, rest) = u32::try_parse(body)?;
                let (atom, _) = u32::try_parse(rest)?;
                if atom == self.state {
                    CapturePoint::ClientState
                } else if atom == self.opacity {
                    CapturePoint::Opacity
                } else {
                    return Ok(());
                }
            }
            (major, composite::NAME_WINDOW_PIXMAP_REQUEST) if major == composite => {
                CapturePoint::Pixmap
            }
            (major, render::CREATE_PICTURE_REQUEST) if major == render => CapturePoint::Picture,
            (major, shape::SELECT_INPUT_REQUEST) if major == shape => CapturePoint::ShapeEvents,
            (major, shape::GET_RECTANGLES_REQUEST) if major == shape => CapturePoint::Shape,
            (major, damage::CREATE_REQUEST) if major == damage => CapturePoint::Damage,
            _ => return Ok(()),
        };
        let (first, rest) = u32::try_parse(body)?;
        match point {
            CapturePoint::Picture => {
                let (drawable, _) = u32::try_parse(rest)?;
                if Some(drawable) != race.resources.pixmap {
                    return Ok(());
                }
                race.resources.picture = Some(first);
            }
            CapturePoint::Damage => {
                let (drawable, _) = u32::try_parse(rest)?;
                if Some(drawable) != race.resources.pixmap {
                    return Ok(());
                }
                race.resources.damage = Some(first);
            }
            CapturePoint::Pixmap => {
                if first != race.window {
                    return Ok(());
                }
                race.resources.pixmap = Some(u32::try_parse(rest)?.0);
            }
            CapturePoint::Attributes
            | CapturePoint::Geometry
            | CapturePoint::ClientEvents
            | CapturePoint::ClientState
            | CapturePoint::ClientTree
            | CapturePoint::ShapeEvents
            | CapturePoint::Shape
            | CapturePoint::Opacity => {
                if first != race.window {
                    return Ok(());
                }
            }
        }
        if point == race.point {
            self.conn.destroy_window(race.window)?.check()?;
            self.completed.send(race.resources)?;
            self.active = None;
        }
        Ok(())
    }
}
