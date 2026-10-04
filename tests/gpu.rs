//! The GPU path against an X server with DRI3 1.2, such as a desktop session: set
//! `COMPUST_GPU_DISPLAY` to its display to run these. They draw only into offscreen pixmaps,
//! so nothing appears on screen; without the variable they are skipped.

use anyhow::{Context, Result, bail, ensure};
use compust_gl::{Device, Dmabuf, Gpu, Mode, Placement, Plane, Rect, fourcc};
use std::{fs::File, os::unix::fs::MetadataExt as _};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        dri3::ConnectionExt as _,
        xproto::{ConnectionExt as _, CreateGCAux, ImageFormat, Pixmap, Rectangle},
    },
    rust_connection::RustConnection,
};

struct Server {
    conn: RustConnection,
    root: u32,
    gpu: Gpu,
}

fn server() -> Result<Option<Server>> {
    let Some(display) = std::env::var_os("COMPUST_GPU_DISPLAY") else {
        eprintln!("skipping: set COMPUST_GPU_DISPLAY to a display with DRI3 1.2");
        return Ok(None);
    };
    let display = display.to_str().context("display name is not UTF-8")?;
    let (conn, screen) = x11rb::connect(Some(display))?;
    let root = conn
        .setup()
        .roots
        .get(screen)
        .context("missing screen")?
        .root;
    let version = conn.dri3_query_version(1, 2)?.reply()?;
    ensure!(
        (version.major_version, version.minor_version) >= (1, 2),
        "the server lacks DRI3 1.2"
    );
    let device = conn.dri3_open(root, NONE)?.reply()?;
    let node = File::from(device.device_fd).metadata()?;
    let gpu = Gpu::open(Device::Drm(node.rdev()))?;
    Ok(Some(Server { conn, root, gpu }))
}

impl Server {
    /// A `size` pixmap of `depth`, filled by the X server with `pixel`.
    fn pixmap(&self, depth: u8, size: u16, pixel: u32) -> Result<Pixmap> {
        let pixmap = self.conn.generate_id()?;
        self.conn
            .create_pixmap(depth, pixmap, self.root, size, size)?
            .check()?;
        let gc = self.conn.generate_id()?;
        self.conn
            .create_gc(gc, pixmap, &CreateGCAux::new().foreground(pixel))?
            .check()?;
        let all = Rectangle {
            x: 0,
            y: 0,
            width: size,
            height: size,
        };
        self.conn.poly_fill_rectangle(pixmap, gc, &[all])?.check()?;
        self.conn.free_gc(gc)?.check()?;
        Ok(pixmap)
    }

    fn dmabuf(&self, pixmap: Pixmap) -> Result<Dmabuf> {
        let reply = self.conn.dri3_buffers_from_pixmap(pixmap)?.reply()?;
        let format = match (reply.depth, reply.bpp) {
            (24, 32) => fourcc::XRGB8888,
            (32, 32) => fourcc::ARGB8888,
            (depth, bpp) => bail!("depth {depth} at {bpp} bits per pixel"),
        };
        let planes = reply
            .buffers
            .into_iter()
            .zip(reply.strides)
            .zip(reply.offsets)
            .map(|((fd, stride), offset)| Plane { fd, offset, stride })
            .collect();
        Ok(Dmabuf {
            width: u32::from(reply.width),
            height: u32::from(reply.height),
            format,
            modifier: reply.modifier,
            planes,
        })
    }

    /// The pixel at `(x, y)` of a depth-24 pixmap, as the X server reads it.
    fn pixel(&self, pixmap: Pixmap, (x, y): (i16, i16)) -> Result<[u8; 3]> {
        let image = self
            .conn
            .get_image(ImageFormat::Z_PIXMAP, pixmap, x, y, 1, 1, u32::MAX)?
            .reply()?;
        let bytes: [u8; 4] = image
            .data
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected a 32-bit pixel"))?;
        let value = u32::from_le_bytes(bytes);
        Ok([
            u8::try_from((value >> 16) & 255)?,
            u8::try_from((value >> 8) & 255)?,
            u8::try_from(value & 255)?,
        ])
    }
}

fn near(left: [u8; 3], right: [u8; 3]) -> bool {
    left.iter().zip(right).all(|(a, b)| a.abs_diff(b) <= 1)
}

#[test]
fn the_gpu_composites_server_pixmaps_into_a_server_pixmap() -> Result<()> {
    let Some(server) = server()? else {
        return Ok(());
    };
    // The server fills both pixmaps; the GPU reads one and draws into the other, and the
    // server reads the result, so each side must see the other's writes.
    let red = server.pixmap(24, 32, 0x00ff_0000)?;
    let back = server.pixmap(24, 32, 0x0000_ff00)?;
    let translucent = server.pixmap(32, 32, 0x8000_0080)?;
    let source = server.gpu.import(&server.dmabuf(red)?)?;
    let veil = server.gpu.import(&server.dmabuf(translucent)?)?;
    let target = server.gpu.import_target(&server.dmabuf(back)?)?;
    let mut frame = server.gpu.frame(&target)?;
    let left = Rect {
        x: 0,
        y: 0,
        width: 16,
        height: 32,
    };
    let right = Rect { x: 16, ..left };
    frame.draw(&source, Placement::At(0, 0), Mode::Over(0.5), &[left])?;
    frame.draw(&veil, Placement::At(0, 0), Mode::Over(1.0), &[right])?;
    server.gpu.flush()?;
    // Half red over green, and half-transparent blue (premultiplied 0x80) over green.
    assert!(
        near(server.pixel(back, (4, 4))?, [128, 127, 0]),
        "{:?}",
        server.pixel(back, (4, 4))?
    );
    assert!(
        near(server.pixel(back, (24, 4))?, [0, 127, 128]),
        "{:?}",
        server.pixel(back, (24, 4))?
    );
    for pixmap in [red, back, translucent] {
        server.conn.free_pixmap(pixmap)?.check()?;
    }
    Ok(())
}

/// Make the server flush its own GPU work, as the GPU painter does before each frame:
/// glamor flushes when a fence is triggered, and the reply shows the trigger was handled.
fn flush_server(server: &Server, fence: u32) -> Result<()> {
    use x11rb::protocol::sync::ConnectionExt as _;
    server.conn.sync_trigger_fence(fence)?;
    server.conn.sync_query_fence(fence)?.reply()?;
    server.conn.sync_reset_fence(fence)?;
    Ok(())
}

fn fresh_pixmaps_read_back(synchronized: bool) -> Result<Option<usize>> {
    use x11rb::protocol::sync::ConnectionExt as _;
    let Some(server) = server()? else {
        return Ok(None);
    };
    server.conn.sync_initialize(3, 1)?.reply()?;
    let fence = server.conn.generate_id()?;
    server
        .conn
        .sync_create_fence(server.root, fence, false)?
        .check()?;
    let copy = server.gpu.target(1, 1)?;
    let area = Rect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    // Another client keeps the server busy, as a desktop's clients do, so it seldom idles,
    // which is when it otherwise flushes its GPU work.
    let display = std::env::var("COMPUST_GPU_DISPLAY")?;
    let busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running = std::sync::Arc::clone(&busy);
    let load = std::thread::spawn(move || -> Result<()> {
        let (conn, screen) = x11rb::connect(Some(&display))?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .context("missing screen")?
            .root;
        let target = conn.generate_id()?;
        conn.create_pixmap(24, target, root, 1024, 1024)?.check()?;
        let gc = conn.generate_id()?;
        conn.create_gc(gc, target, &CreateGCAux::new().foreground(0x0000_00ff))?
            .check()?;
        let all = Rectangle {
            x: 0,
            y: 0,
            width: 1024,
            height: 1024,
        };
        while running.load(std::sync::atomic::Ordering::Relaxed) {
            for _ in 0..64 {
                conn.poly_fill_rectangle(target, gc, &[all])?;
            }
            conn.get_input_focus()?.reply()?;
        }
        Ok(())
    });
    let mut stale = 0;
    for _ in 0..100 {
        // Sharing a pixmap can move it into a new buffer with a copy the server has not yet
        // sent to its GPU, so a read right after the reply may find the new buffer empty.
        let pixmap = server.pixmap(24, 256, 0x00ff_0000)?;
        let texture = server.gpu.import(&server.dmabuf(pixmap)?)?;
        if synchronized {
            flush_server(&server, fence)?;
        }
        server.gpu.frame(&copy)?.draw(
            &texture,
            Placement::At(-128, -128),
            Mode::Replace,
            &[area],
        )?;
        if server.gpu.read(&copy, area)?.first() != Some(&255) {
            stale += 1;
        }
        server.conn.free_pixmap(pixmap)?;
    }
    busy.store(false, std::sync::atomic::Ordering::Relaxed);
    load.join()
        .map_err(|_| anyhow::anyhow!("the load thread panicked"))??;
    Ok(Some(stale))
}

#[test]
fn the_server_flushes_its_gpu_work_when_a_fence_triggers() -> Result<()> {
    let Some(stale) = fresh_pixmaps_read_back(true)? else {
        return Ok(());
    };
    assert_eq!(stale, 0, "the GPU read pixmaps the server had not finished");
    Ok(())
}
