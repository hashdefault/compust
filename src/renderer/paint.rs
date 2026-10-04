use super::{
    Renderer, blur,
    damage::{self, Shown},
};
use crate::{
    animation::multiply_alpha,
    config::Config,
    region::{Rect, Region},
    scene::Scene,
    session::Session,
    surface::Surface,
};
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
    /// Repaint and show what changed since the last frame; reports whether anything did.
    pub(crate) fn paint(
        &mut self,
        session: &Session,
        scene: &Scene,
        config: &Config,
    ) -> Result<bool> {
        let now = Instant::now();
        let global = u16::try_from(u32::from(config.opacity) * u32::from(u16::MAX) / 100)?;
        let visible: Vec<_> = scene
            .windows
            .iter()
            .filter_map(|surface| {
                let opacity = multiply_alpha(
                    multiply_alpha(surface.opacity, global),
                    surface.fade.sample(now),
                );
                (opacity > 0).then_some((surface, opacity))
            })
            .collect();
        let shown: Vec<_> = visible
            .iter()
            .map(|&(surface, opacity)| Shown::new(surface, opacity))
            .collect();
        let mut changed = std::mem::take(&mut self.damage);
        damage::changes(&self.shown, &shown, &mut changed);
        self.shown = shown;
        let mut area = changed.within(self.screen());
        let footprints: Vec<_> = visible
            .iter()
            .filter(|&&(surface, opacity)| self.blurs(surface, opacity))
            .filter_map(|(surface, _)| blur::footprint(surface, self.size, self.levels.len()))
            .collect();
        damage::spread(&mut area, &footprints);
        if area.is_empty() {
            return Ok(false);
        }
        let rects = area.x11()?;
        self.paint_background(session, &rects)?;
        for (surface, opacity) in visible {
            self.paint_surface(session, surface, opacity, &area)?;
        }
        session.conn.render_set_picture_clip_rectangles(
            self.back.id,
            0,
            0,
            &[self.screen().x11()?],
        )?;
        self.submit(session, &rects)?;
        Ok(true)
    }

    fn blurs(&self, surface: &Surface, opacity: u16) -> bool {
        !self.levels.is_empty() && (opacity < u16::MAX || surface.has_alpha)
    }

    fn paint_background(&self, session: &Session, area: &[Rectangle]) -> Result<()> {
        let conn = &session.conn;
        let full = self.screen().x11()?;
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, area)?;
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
        Ok(())
    }

    fn paint_surface(
        &self,
        session: &Session,
        surface: &Surface,
        opacity: u16,
        area: &Region,
    ) -> Result<()> {
        let conn = &session.conn;
        let bounds = surface.bounds();
        let origin = (surface.geometry.x, surface.geometry.y);
        let clip = area.clip(
            surface
                .shape
                .iter()
                .filter_map(|rect| Rect::at(*rect, origin).intersect(bounds)),
        )?;
        if clip.is_empty() {
            return Ok(());
        }
        if self.blurs(surface, opacity) {
            self.blur(session, surface, &clip)?;
        }
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &clip)?;
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
        Ok(())
    }
}
