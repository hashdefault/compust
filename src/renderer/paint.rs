use super::Renderer;
use crate::{animation::multiply_alpha, config::Config, scene::Scene, session::Session};
use anyhow::Result;
use std::time::Instant;
use x11rb::{
    NONE,
    protocol::{
        render::{Color, ConnectionExt as _, PictOp},
        xproto::Rectangle,
    },
};
impl Renderer {
    pub(crate) fn paint(
        &mut self,
        session: &Session,
        scene: &Scene,
        config: &Config,
    ) -> Result<()> {
        let conn = &session.conn;
        let full = Rectangle {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        };
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &[full])?;
        conn.render_fill_rectangles(
            PictOp::SRC,
            self.back.id,
            Color {
                red: 0x1818,
                green: 0x1818,
                blue: 0x2020,
                alpha: u16::MAX,
            },
            &[full],
        )?;
        if let Some(wallpaper) = &self.wallpaper {
            conn.render_composite(
                PictOp::SRC,
                wallpaper.id,
                NONE,
                self.back.id,
                0,
                0,
                0,
                0,
                0,
                0,
                full.width,
                full.height,
            )?;
        }
        let now = Instant::now();
        let global = u16::try_from(u32::from(config.opacity) * u32::from(u16::MAX) / 100)?;
        for surface in &scene.windows {
            let opacity = multiply_alpha(
                multiply_alpha(surface.opacity, global),
                surface.fade.sample(now),
            );
            if opacity == 0 {
                continue;
            }
            if !self.horizontal.is_empty() && (opacity < u16::MAX || surface.has_alpha) {
                self.blur(session, surface)?;
            }
            conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &surface.shape)?;
            conn.render_fill_rectangles(
                PictOp::SRC,
                self.alpha.id,
                Color {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: opacity,
                },
                &[Rectangle {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                }],
            )?;
            conn.render_composite(
                PictOp::OVER,
                surface.picture.id,
                self.alpha.id,
                self.back.id,
                0,
                0,
                0,
                0,
                surface.geometry.x,
                surface.geometry.y,
                surface.size.width,
                surface.size.height,
            )?;
        }
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &[full])?;
        self.submit(session)?;
        Ok(())
    }
}
