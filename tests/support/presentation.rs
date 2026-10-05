use super::proxy::Proxy;
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    time::Duration,
};
use x11rb::{
    NONE,
    connection::{Connection, RequestConnection},
    protocol::{present, sync::ConnectionExt as _, xfixes, xproto::ConnectionExt as _},
    rust_connection::RustConnection,
};

/// Rectangles as `(x, y, width, height)`.
pub(crate) type Area = Vec<(i16, i16, u16, u16)>;

pub(crate) enum Fault {
    /// An incompatible pixmap: the server rejects the submission with `BadMatch`.
    Depth,
    Pixmap,
    Window,
    /// A wait fence that stays untriggered until `release`: neither completion nor idle
    /// events follow before it.
    Fence,
}

pub(crate) struct Presentation {
    proxy: Proxy,
    commands: SyncSender<Fault>,
    intercepted: Receiver<()>,
    submissions: Arc<AtomicU32>,
    /// Each submission's update region; `None` updates the whole window.
    updates: Arc<Mutex<Vec<Option<Area>>>>,
    fence: u32,
    conn: RustConnection,
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
        let xfixes_opcode = conn
            .extension_information(xfixes::X11_EXTENSION_NAME)?
            .context("missing XFixes extension")?
            .major_opcode;
        let incompatible = conn.generate_id()?;
        conn.create_pixmap(8, incompatible, root, 1, 1)?.check()?;
        conn.sync_initialize(3, 1)?.reply()?;
        let fence = conn.generate_id()?;
        conn.sync_create_fence(root, fence, false)?.check()?;
        let (commands, receive) = sync_channel(1);
        let (send, intercepted) = sync_channel(1);
        let submissions = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&submissions);
        let updates = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&updates);
        let mut regions = HashMap::new();
        let (display, proxy) = Proxy::start(display, move |header, body| {
            if header.major_opcode == xfixes_opcode
                && [xfixes::CREATE_REGION_REQUEST, xfixes::SET_REGION_REQUEST]
                    .contains(&header.minor_opcode)
            {
                regions.insert(
                    word(body, 0)?,
                    rectangles(body.get(4..).unwrap_or_default())?,
                );
            }
            if header.major_opcode == opcode && header.minor_opcode == present::PIXMAP_REQUEST {
                count.fetch_add(1, Ordering::Relaxed);
                let update = regions.get(&word(body, 16)?).cloned();
                record
                    .lock()
                    .map_err(|_| anyhow::anyhow!("update record poisoned"))?
                    .push(update);
                if let Ok(fault) = receive.try_recv() {
                    let (offset, replacement) = match fault {
                        Fault::Depth => (4, incompatible),
                        Fault::Pixmap => (4, NONE),
                        Fault::Window => (0, NONE),
                        Fault::Fence => (28, fence),
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
                updates,
                fence,
                conn,
            },
        ))
    }

    /// The update region of every submission so far, in order.
    pub(crate) fn updates(&self) -> Result<Vec<Option<Area>>> {
        Ok(self
            .updates
            .lock()
            .map_err(|_| anyhow::anyhow!("update record poisoned"))?
            .clone())
    }

    pub(crate) fn inject(&self, fault: Fault) -> Result<()> {
        self.commands.send(fault)?;
        Ok(())
    }

    pub(crate) fn injected(&self) -> Result<u32> {
        self.intercepted
            .recv_timeout(Duration::from_secs(5))
            .context("Present request was not intercepted")?;
        Ok(self.submissions.load(Ordering::Relaxed))
    }

    /// Trigger the fence of `Fault::Fence`, so the server shows the submission it held back.
    pub(crate) fn release(&self) -> Result<()> {
        self.conn.sync_trigger_fence(self.fence)?.check()?;
        Ok(())
    }

    pub(crate) fn submissions(&self) -> u32 {
        self.submissions.load(Ordering::Relaxed)
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.proxy.finish()
    }
}

fn word(body: &[u8], offset: usize) -> Result<u32> {
    let bytes = body
        .get(offset..offset + 4)
        .context("short request")?
        .try_into()?;
    Ok(u32::from_ne_bytes(bytes))
}

fn rectangles(bytes: &[u8]) -> Result<Area> {
    bytes
        .chunks_exact(8)
        .map(|chunk| {
            let [x0, x1, y0, y1, w0, w1, h0, h1] = <[u8; 8]>::try_from(chunk)?;
            Ok((
                i16::from_ne_bytes([x0, x1]),
                i16::from_ne_bytes([y0, y1]),
                u16::from_ne_bytes([w0, w1]),
                u16::from_ne_bytes([h0, h1]),
            ))
        })
        .collect()
}
