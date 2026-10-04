use super::{Desktop, next_event};
use anyhow::{Context, Result};
use std::{
    io::Write,
    time::{Duration, Instant},
};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        damage::ConnectionExt as _,
        xproto::{ConnectionExt as _, Drawable, ImageFormat, ImageOrder, MapState},
    },
};
impl Desktop {
    /// The pixel the compositor shows at `point`, read from its overlay.
    pub(crate) fn pixel(&self, point: (i16, i16)) -> Result<[u8; 3]> {
        self.pixel_of(self.overlay, point)
    }

    /// The pixel the screen shows at `point`, read from the root: what windows draw there
    /// themselves while compositing is suspended.
    pub(crate) fn screen_pixel(&self, point: (i16, i16)) -> Result<[u8; 3]> {
        self.pixel_of(self.root, point)
    }

    /// Whether the compositor has suspended compositing, which takes its overlay off the
    /// screen.
    pub(crate) fn suspended(&self) -> Result<bool> {
        let overlay = self.conn.get_window_attributes(self.overlay)?.reply()?;
        Ok(overlay.map_state == MapState::UNMAPPED)
    }

    /// Wait until compositing is suspended, or composited again.
    pub(crate) fn until_suspended(&self, suspended: bool) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.suspended()? != suspended {
            anyhow::ensure!(
                Instant::now() < deadline,
                "compositing stayed {}",
                if suspended { "on" } else { "suspended" }
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    fn pixel_of(&self, drawable: Drawable, point: (i16, i16)) -> Result<[u8; 3]> {
        let reply = self
            .conn
            .get_image(
                ImageFormat::Z_PIXMAP,
                drawable,
                point.0,
                point.1,
                1,
                1,
                u32::MAX,
            )?
            .reply()?;
        let bytes: [u8; 4] = reply
            .data
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 32-bit Xvfb pixel"))?;
        let value = if self.conn.setup().image_byte_order == ImageOrder::LSB_FIRST {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        };
        Ok([
            u8::try_from((value >> 16) & 255)?,
            u8::try_from((value >> 8) & 255)?,
            u8::try_from(value & 255)?,
        ])
    }

    pub(crate) fn until_pixel(
        &self,
        point: (i16, i16),
        accept: impl Fn([u8; 3]) -> bool,
    ) -> Result<[u8; 3]> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.conn.damage_subtract(self.damage, NONE, NONE)?;
            let pixel = self.pixel(point)?;
            if accept(pixel) {
                return Ok(pixel);
            }
            self.conn.flush()?;
            next_event(&self.conn, deadline)
                .with_context(|| format!("pixel at {point:?} remained {pixel:?}"))?;
        }
    }

    /// The whole overlay as 32-bit pixels.
    pub(crate) fn image(&self) -> Result<Vec<u8>> {
        let geometry = self.conn.get_geometry(self.overlay)?.reply()?;
        Ok(self
            .conn
            .get_image(
                ImageFormat::Z_PIXMAP,
                self.overlay,
                0,
                0,
                geometry.width,
                geometry.height,
                u32::MAX,
            )?
            .reply()?
            .data)
    }

    pub(crate) fn screenshot(&self, name: &str) -> Result<()> {
        if let Some(directory) = std::env::var_os("COMPUST_ARTIFACTS") {
            std::fs::create_dir_all(&directory)?;
            let path = std::path::PathBuf::from(directory).join(format!("{name}.ppm"));
            let mut output = std::io::BufWriter::new(std::fs::File::create(path)?);
            let geometry = self.conn.get_geometry(self.overlay)?.reply()?;
            write!(output, "P6\n{} {}\n255\n", geometry.width, geometry.height)?;
            let reply = self
                .conn
                .get_image(
                    ImageFormat::Z_PIXMAP,
                    self.overlay,
                    0,
                    0,
                    geometry.width,
                    geometry.height,
                    u32::MAX,
                )?
                .reply()?;
            for bytes in reply.data.chunks_exact(4) {
                let word: [u8; 4] = bytes.try_into()?;
                let pixel = if self.conn.setup().image_byte_order == ImageOrder::LSB_FIRST {
                    u32::from_le_bytes(word)
                } else {
                    u32::from_be_bytes(word)
                };
                output.write_all(&[
                    u8::try_from((pixel >> 16) & 255)?,
                    u8::try_from((pixel >> 8) & 255)?,
                    u8::try_from(pixel & 255)?,
                ])?;
            }
            output.flush()?;
        }
        Ok(())
    }
}
