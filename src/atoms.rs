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
        })
    }
}

pub(crate) fn cardinal(conn: &RustConnection, window: Window, atom: Atom) -> Result<Option<u32>> {
    let reply = conn
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)?
        .reply()?;
    Ok(reply.value32().and_then(|mut values| values.next()))
}
