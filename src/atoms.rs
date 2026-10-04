use crate::rules::WindowType;
use anyhow::Result;
use x11rb::{
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window},
    rust_connection::RustConnection,
};

#[derive(Debug)]
pub(crate) struct Atoms {
    pub(crate) selection: Atom,
    pub(crate) manager: Atom,
    pub(crate) opacity: Atom,
    pub(crate) wallpaper: [Atom; 2],
    pub(crate) wm_state: Atom,
    pub(crate) timestamp: Atom,
    pub(crate) net_wm_name: Atom,
    pub(crate) utf8_string: Atom,
    pub(crate) window_type: Atom,
    /// Each `_NET_WM_WINDOW_TYPE_` atom with the type it names.
    pub(crate) window_types: Vec<(Atom, WindowType)>,
}

impl Atoms {
    pub(crate) fn new(conn: &RustConnection, screen: usize) -> Result<Self> {
        let intern =
            |name: &[u8]| -> Result<Atom> { Ok(conn.intern_atom(false, name)?.reply()?.atom) };
        Ok(Self {
            selection: intern(format!("_NET_WM_CM_S{screen}").as_bytes())?,
            manager: intern(b"MANAGER")?,
            opacity: intern(b"_NET_WM_WINDOW_OPACITY")?,
            wallpaper: [intern(b"_XROOTPMAP_ID")?, intern(b"ESETROOT_PMAP_ID")?],
            wm_state: intern(b"WM_STATE")?,
            timestamp: intern(b"_COMPUST_TIMESTAMP")?,
            net_wm_name: intern(b"_NET_WM_NAME")?,
            utf8_string: intern(b"UTF8_STRING")?,
            window_type: intern(b"_NET_WM_WINDOW_TYPE")?,
            window_types: {
                let cookies = WindowType::ALL
                    .iter()
                    .map(|(kind, name)| Ok((*kind, conn.intern_atom(false, name.as_bytes())?)))
                    .collect::<Result<Vec<_>>>()?;
                cookies
                    .into_iter()
                    .map(|(kind, cookie)| Ok((cookie.reply()?.atom, kind)))
                    .collect::<Result<_>>()?
            },
        })
    }

    /// The properties rules match on, whose changes refresh a window's identity.
    pub(crate) fn identifies(&self, atom: Atom) -> bool {
        [
            AtomEnum::WM_CLASS,
            AtomEnum::WM_NAME,
            AtomEnum::WM_TRANSIENT_FOR,
        ]
        .into_iter()
        .any(|known| atom == u32::from(known))
            || atom == self.net_wm_name
            || atom == self.window_type
    }
}

pub(crate) fn cardinal(conn: &RustConnection, window: Window, atom: Atom) -> Result<Option<u32>> {
    let reply = conn
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)?
        .reply()?;
    if reply.type_ != u32::from(AtomEnum::CARDINAL)
        || reply.format != 32
        || reply.value_len != 1
        || reply.bytes_after != 0
    {
        return Ok(None);
    }
    Ok(reply.value32().and_then(|mut values| values.next()))
}
