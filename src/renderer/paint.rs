use super::{
    Renderer, Source, blur,
    damage::{self, Blurred, Change, OUTPUT, Shown},
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
        xproto::{Rectangle, Window},
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
        let mut changes = self.changes(&visible);
        let screen = self.screen();
        let mut area = Region::default();
        for change in changes.iter().filter(|change| change.shown) {
            if let Some(rect) = change.area.intersect(screen) {
                area.add(rect);
            }
        }
        let (windows, blurred): (Vec<_>, Vec<_>) = visible
            .iter()
            .enumerate()
            .filter(|&(_, &(surface, opacity))| self.blurs(surface, opacity))
            .filter_map(|(index, (surface, _))| {
                let bounds = surface.bounds().intersect(screen)?;
                let kept = self
                    .backdrops
                    .iter()
                    .any(|kept| kept.window == surface.window && kept.area == bounds);
                let blurred = Blurred {
                    layer: index + 1,
                    bounds,
                    footprint: blur::footprint(surface, self.size, self.levels.len())?,
                    kept,
                };
                Some((surface.window, blurred))
            })
            .unzip();
        let fresh = damage::plan(&blurred, &mut changes, &mut area);
        self.backdrops.retain(|kept| windows.contains(&kept.window));
        if area.is_empty() {
            return Ok(false);
        }
        let rects = area.x11()?;
        self.paint_background(session, &rects)?;
        for (surface, opacity) in visible {
            let blur = windows
                .iter()
                .zip(blurred.iter().zip(&fresh))
                .find(|(window, _)| **window == surface.window)
                .map(|(_, (blurred, fresh))| (blurred.bounds, *fresh));
            self.paint_surface(session, (surface, opacity), &area, blur)?;
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

    /// What changed since the last frame, which `visible` replaces: the surfaces' own changes
    /// and those reported since, at the layer each belongs to.
    fn changes(&mut self, visible: &[(&Surface, u16)]) -> Vec<Change> {
        let shown: Vec<_> = visible
            .iter()
            .map(|&(surface, opacity)| Shown::new(surface, opacity))
            .collect();
        let mut changes = damage::changes(&self.shown, &shown);
        self.shown = shown;
        for (source, region) in std::mem::take(&mut self.pending) {
            let layer = match source {
                Source::Background => Some(0),
                Source::Window(window) => visible
                    .iter()
                    .position(|(surface, _)| surface.window == window)
                    .map(|index| index + 1),
                Source::Output => Some(OUTPUT),
            };
            if let Some(layer) = layer {
                changes.extend(region.rects().iter().map(|&area| Change {
                    area,
                    layer,
                    shown: true,
                }));
            }
        }
        changes
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

    /// Paint `surface` at its opacity within `area`. A blurred surface first shows its
    /// backdrop over `blur`'s bounds, blurred again when `blur` says it is due.
    fn paint_surface(
        &mut self,
        session: &Session,
        (surface, opacity): (&Surface, u16),
        area: &Region,
        blur: Option<(Rect, bool)>,
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
        // A backdrop due to blur again is refreshed even where none of it shows, so that a
        // later frame never reuses a stale one.
        if let Some((bounds, true)) = blur {
            self.keep(surface.window, bounds)?;
        }
        let backdrop = self.backdrop(surface.window, blur);
        if let (Some(backdrop), Some((_, true))) = (backdrop, blur) {
            self.blur(session, surface, backdrop)?;
        }
        if clip.is_empty() {
            return Ok(());
        }
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &clip)?;
        if let Some(backdrop) = backdrop {
            let area = backdrop.area.x11()?;
            conn.render_composite(
                PictOp::SRC,
                backdrop.picture.id,
                NONE,
                self.back.id,
                0,
                0,
                0,
                0,
                area.x,
                area.y,
                area.width,
                area.height,
            )?;
        }
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

    /// The kept backdrop a blurred surface shows over `blur`'s bounds.
    fn backdrop(&self, window: Window, blur: Option<(Rect, bool)>) -> Option<&blur::Backdrop> {
        let (bounds, _) = blur?;
        self.backdrops
            .iter()
            .find(|kept| kept.window == window && kept.area == bounds)
    }
}
