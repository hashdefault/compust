use anyhow::Result;
use x11rb::{
    NONE,
    errors::ReplyError,
    protocol::{
        ErrorKind,
        xproto::{Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, Window},
    },
    rust_connection::RustConnection,
};

pub(super) struct ClientTree {
    pub(super) client: Window,
    pub(super) watched: Vec<Window>,
}

impl ClientTree {
    pub(super) fn discover(conn: &RustConnection, frame: Window, state: Atom) -> Result<Self> {
        let mut tree = Self {
            client: frame,
            watched: Vec::new(),
        };
        let mut pending = vec![(frame, 0_u8)];
        while let Some((window, depth)) = pending.pop() {
            let result = (|| -> Result<bool> {
                conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new()
                        .event_mask(EventMask::PROPERTY_CHANGE | EventMask::SUBSTRUCTURE_NOTIFY),
                )?
                .check()?;
                let property = conn
                    .get_property(false, window, state, AtomEnum::ANY, 0, 0)?
                    .reply()?;
                if property.type_ != NONE {
                    return Ok(true);
                }
                if depth < 8 {
                    pending.extend(
                        conn.query_tree(window)?
                            .reply()?
                            .children
                            .into_iter()
                            .map(|child| (child, depth.saturating_add(1))),
                    );
                }
                Ok(false)
            })();
            match result {
                Ok(found) => {
                    tree.watched.push(window);
                    if found {
                        tree.client = window;
                        break;
                    }
                }
                Err(error) if window != frame && window_gone(&error) => (),
                Err(error) => return Err(error),
            }
        }
        Ok(tree)
    }
}

pub(super) fn window_gone(error: &anyhow::Error) -> bool {
    matches!(error.downcast_ref::<ReplyError>(), Some(ReplyError::X11Error(error))
        if error.error_kind == ErrorKind::Window)
}
