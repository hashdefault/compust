use super::Renderer;
use crate::{session::Session, surface::Surface};
use anyhow::Result;
use x11rb::{
    NONE,
    protocol::{
        render::{ConnectionExt as _, PictOp},
        xproto::Rectangle,
    },
};
impl Renderer {
    pub(super) fn blur(&self, session: &Session, surface: &Surface) -> Result<()> {
        let conn = &session.conn;
        let full = Rectangle {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        };
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &[full])?;
        conn.render_set_picture_filter(self.back.id, b"convolution", &self.horizontal)?;
        conn.render_composite(
            PictOp::SRC,
            self.back.id,
            NONE,
            self.scratch.id,
            0,
            0,
            0,
            0,
            0,
            0,
            full.width,
            full.height,
        )?;
        conn.render_set_picture_filter(self.back.id, b"nearest", &[])?;
        conn.render_set_picture_clip_rectangles(
            self.back.id,
            surface.geometry.x,
            surface.geometry.y,
            &surface.shape,
        )?;
        conn.render_set_picture_filter(self.scratch.id, b"convolution", &self.vertical)?;
        conn.render_composite(
            PictOp::SRC,
            self.scratch.id,
            NONE,
            self.back.id,
            surface.geometry.x,
            surface.geometry.y,
            0,
            0,
            surface.geometry.x,
            surface.geometry.y,
            surface.size.width,
            surface.size.height,
        )?;
        Ok(())
    }
}
pub(super) fn kernel(radius: u8) -> (Vec<i32>, Vec<i32>) {
    if radius == 0 {
        return (Vec::new(), Vec::new());
    }
    let width = i32::from(radius) * 2 + 1;
    let mut weights = vec![65536 / width; usize::from(radius) * 2 + 1];
    if let Some(center) = weights.get_mut(usize::from(radius)) {
        *center += 65536 % width;
    }
    let mut horizontal = vec![width * 65536, 65536];
    let mut vertical = vec![65536, width * 65536];
    horizontal.extend_from_slice(&weights);
    vertical.extend_from_slice(&weights);
    (horizontal, vertical)
}
