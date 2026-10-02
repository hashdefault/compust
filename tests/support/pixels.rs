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
        xproto::{ConnectionExt as _, ImageFormat, ImageOrder},
    },
};
impl Desktop {
    pub(crate) fn pixel(&self, point: (i16, i16)) -> Result<[u8; 3]> {
        let reply = self
            .conn
            .get_image(
                ImageFormat::Z_PIXMAP,
                self.overlay,
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

    pub(crate) fn screenshot(&self, name: &str) -> Result<()> {
        if let Some(directory) = std::env::var_os("COMPUST_ARTIFACTS") {
            std::fs::create_dir_all(&directory)?;
            let path = std::path::PathBuf::from(directory).join(format!("{name}.ppm"));
            let mut output = std::io::BufWriter::new(std::fs::File::create(path)?);
            write!(output, "P6\n320 240\n255\n")?;
            let reply = self
                .conn
                .get_image(
                    ImageFormat::Z_PIXMAP,
                    self.overlay,
                    0,
                    0,
                    320,
                    240,
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
