use super::proxy::Proxy;
use anyhow::{Context, Result};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    time::Duration,
};
use x11rb::{
    NONE,
    connection::{Connection, RequestConnection},
    protocol::{present, xproto::ConnectionExt as _},
    rust_connection::RustConnection,
};

pub(crate) enum Fault {
    Depth,
    Pixmap,
    Window,
}

pub(crate) struct Presentation {
    proxy: Proxy,
    commands: SyncSender<Fault>,
    intercepted: Receiver<()>,
    submissions: Arc<AtomicU32>,
    _conn: RustConnection,
}

impl Presentation {
    pub(crate) fn start(display: &str) -> Result<(String, Self)> {
        let (conn, screen) = x11rb::connect(Some(display))?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .context("missing screen")?
            .root;
        let opcode = conn
            .extension_information(present::X11_EXTENSION_NAME)?
            .context("missing Present extension")?
            .major_opcode;
        let incompatible = conn.generate_id()?;
        conn.create_pixmap(8, incompatible, root, 1, 1)?.check()?;
        let (commands, receive) = sync_channel(1);
        let (send, intercepted) = sync_channel(1);
        let submissions = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&submissions);
        let (display, proxy) = Proxy::start(display, move |header, body| {
            if header.major_opcode == opcode && header.minor_opcode == present::PIXMAP_REQUEST {
                count.fetch_add(1, Ordering::Relaxed);
                if let Ok(fault) = receive.try_recv() {
                    let (offset, replacement) = match fault {
                        Fault::Depth => (4, incompatible),
                        Fault::Pixmap => (4, NONE),
                        Fault::Window => (0, NONE),
                    };
                    body.get_mut(offset..offset + 4)
                        .context("short Present request")?
                        .copy_from_slice(&replacement.to_ne_bytes());
                    send.send(())?;
                }
            }
            Ok(())
        })?;
        Ok((
            display,
            Self {
                proxy,
                commands,
                intercepted,
                submissions,
                _conn: conn,
            },
        ))
    }

    pub(crate) fn reject_next(&self, fault: Fault) -> Result<()> {
        self.commands.send(fault)?;
        Ok(())
    }

    pub(crate) fn rejected(&self) -> Result<u32> {
        self.intercepted
            .recv_timeout(Duration::from_secs(5))
            .context("Present request was not intercepted")?;
        Ok(self.submissions.load(Ordering::Relaxed))
    }

    pub(crate) fn submissions(&self) -> u32 {
        self.submissions.load(Ordering::Relaxed)
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.proxy.finish()
    }
}
