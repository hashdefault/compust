use super::Surface;
use anyhow::{Result, ensure};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use x11rb::{
    connection::Connection,
    protocol::xproto::{ConnectionExt as _, ImageFormat, ImageOrder},
};

impl Surface {
    pub(in super::super) fn pixel(&self, point: (i16, i16)) -> Result<u32> {
        let image = self
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
        Ok(self.rgb(image.data.as_slice().try_into()?))
    }

    pub(in super::super) fn window_has_color(&self, window: u32, color: u32) -> Result<bool> {
        Ok(self.window_pixel(window)? == color)
    }

    /// The composited pixel at the center of `window`.
    pub(in super::super) fn window_pixel(&self, window: u32) -> Result<u32> {
        let geometry = self.conn.get_geometry(window)?.reply()?;
        let position = self
            .conn
            .translate_coordinates(
                window,
                self.root,
                i16::try_from(geometry.width / 2)?,
                i16::try_from(geometry.height / 2)?,
            )?
            .reply()?;
        self.pixel((position.dst_x, position.dst_y))
    }

    fn rgb(&self, bytes: [u8; 4]) -> u32 {
        let word = if self.conn.setup().image_byte_order == ImageOrder::LSB_FIRST {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        };
        word & 0x00ff_ffff
    }
}

pub(super) fn screenshot(surface: &Surface, path: &Path) -> Result<()> {
    let image = surface
        .conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            surface.overlay,
            0,
            0,
            surface.width,
            surface.height,
            u32::MAX,
        )?
        .reply()?;
    ensure!(
        image.data.len() == usize::from(surface.width) * usize::from(surface.height) * 4,
        "unexpected screenshot format"
    );
    let mut output = BufWriter::new(File::create(path)?);
    write!(output, "P6\n{} {}\n255\n", surface.width, surface.height)?;
    for bytes in image.data.chunks_exact(4) {
        let pixel = surface.rgb(bytes.try_into()?);
        output.write_all(&[
            u8::try_from((pixel >> 16) & 255)?,
            u8::try_from((pixel >> 8) & 255)?,
            u8::try_from(pixel & 255)?,
        ])?;
    }
    output.flush()?;
    Ok(())
}
