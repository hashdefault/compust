use super::{
    Renderer, Source, blur,
    cover::{self, Layer},
    damage::{self, Blurred, Change, OUTPUT, Shown},
};
use crate::{
    animation::multiply_alpha,
    capabilities,
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

/// What a surface shows beneath it in a frame.
#[derive(Clone, Copy)]
pub(super) enum Beneath {
    /// The scene as painted, because the surface is not blurred.
    Scene,
    /// Its kept backdrop, which covers these screen bounds.
    Kept(Rect),
    /// Its backdrop blurred again over these screen bounds, from the scene in its footprint.
    Fresh { bounds: Rect, footprint: Rect },
}

/// A surface a frame paints.
pub(super) struct Part<'a> {
    pub(super) surface: &'a Surface,
    pub(super) opacity: u16,
    /// Where it shows in the repaint area; empty where it shows nowhere.
    pub(super) clip: Vec<Rect>,
    pub(super) beneath: Beneath,
}

/// What a frame paints, which either painter carries out.
pub(super) struct Plan<'a> {
    /// Where the background shows in the repaint area.
    pub(super) background: Vec<Rect>,
    /// The surfaces, bottom to top, without those hidden whole.
    pub(super) parts: Vec<Part<'a>>,
}

impl Renderer {
    /// Repaint and show what changed since the last frame; reports whether anything did.
    pub(crate) fn paint(
        &mut self,
        session: &Session,
        scene: &Scene,
        config: &Config,
    ) -> Result<bool> {
        let visible = visible(scene, config)?;
        let Some((area, plan)) = self.plan(&visible) else {
            return Ok(false);
        };
        let wallpaper = self.wallpaper_pixmap;
        let failed = if let Some(gpu) = self.gpu.as_mut() {
            gpu.paint(&plan, wallpaper).err()
        } else {
            self.paint_xrender(session, &plan)?;
            None
        };
        if let Some(error) = failed {
            tracing::warn!("GPU rendering failed; continuing with XRender: {error:#}");
            session.capabilities.gpu.set(capabilities::Gpu::Failed);
            self.gpu = None;
            self.invalidate();
            return self.paint(session, scene, config);
        }
        self.submit(session, &area.x11()?)?;
        Ok(true)
    }

    /// Find what changed and plan the frame that shows it, with the area it repaints; `None`
    /// when nothing changed.
    fn plan<'a>(&mut self, visible: &[(&'a Surface, u16)]) -> Option<(Region, Plan<'a>)> {
        let mut changes = self.changes(visible);
        let mut area = Region::default();
        for change in changes.iter().filter(|change| change.shown) {
            if let Some(rect) = change.area.intersect(self.screen()) {
                area.add(rect);
            }
        }
        let (windows, blurred) = self.blurred(visible);
        let fresh = damage::plan(&blurred, &mut changes, &mut area);
        self.backdrops.retain(|kept| windows.contains(&kept.window));
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.retain(&windows);
        }
        if area.is_empty() {
            return None;
        }
        let beneath: Vec<_> = visible
            .iter()
            .map(|(surface, _)| {
                let index = windows.iter().position(|window| *window == surface.window);
                match index.and_then(|index| Some((blurred.get(index)?, *fresh.get(index)?))) {
                    Some((blurred, true)) => Beneath::Fresh {
                        bounds: blurred.bounds,
                        footprint: blurred.footprint,
                    },
                    Some((blurred, false)) => Beneath::Kept(blurred.bounds),
                    None => Beneath::Scene,
                }
            })
            .collect();
        let layers: Vec<_> = visible
            .iter()
            .zip(&beneath)
            .map(|(&(surface, opacity), beneath)| Layer {
                opaque: self.opaque(surface, opacity),
                blur: match *beneath {
                    Beneath::Fresh { bounds, footprint } => Some((bounds, footprint)),
                    _ => None,
                },
            })
            .collect();
        let covers = cover::covers(&layers);
        let mut parts = Vec::with_capacity(visible.len());
        for (index, (&(surface, opacity), beneath)) in visible.iter().zip(beneath).enumerate() {
            if covers.hidden.get(index) == Some(&true) {
                // Hidden whole, it need not blur, and its backdrop would go stale.
                self.backdrops.retain(|kept| kept.window != surface.window);
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.forget(surface.window);
                }
                continue;
            }
            let clip = area.clip(shape(surface));
            let clip = match covers.above.get(index) {
                Some(cover) => cover.visible(clip),
                None => clip,
            };
            parts.push(Part {
                surface,
                opacity,
                clip,
                beneath,
            });
        }
        let background = covers.background.visible(area.rects().to_vec());
        Some((area, Plan { background, parts }))
    }

    fn paint_xrender(&mut self, session: &Session, plan: &Plan<'_>) -> Result<()> {
        self.paint_background(session, &plan.background)?;
        for part in &plan.parts {
            self.paint_surface(
                session,
                (part.surface, part.opacity),
                &part.clip,
                part.beneath,
            )?;
        }
        session.conn.render_set_picture_clip_rectangles(
            self.back.id,
            0,
            0,
            &[self.screen().x11()?],
        )?;
        Ok(())
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

    /// The blurred surfaces among `visible` and their windows, bottom to top.
    fn blurred(&self, visible: &[(&Surface, u16)]) -> (Vec<Window>, Vec<Blurred>) {
        visible
            .iter()
            .enumerate()
            .filter(|&(_, &(surface, opacity))| self.blurs(surface, opacity))
            .filter_map(|(index, (surface, _))| {
                let bounds = surface.bounds().intersect(self.screen())?;
                let kept = match &self.gpu {
                    Some(gpu) => gpu.kept(surface.window, bounds),
                    None => self
                        .backdrops
                        .iter()
                        .any(|kept| kept.window == surface.window && kept.area == bounds),
                };
                let blurred = Blurred {
                    layer: index + 1,
                    bounds,
                    footprint: blur::footprint(surface, self.size, self.levels.len())?,
                    kept,
                };
                Some((surface.window, blurred))
            })
            .unzip()
    }

    fn blurs(&self, surface: &Surface, opacity: u16) -> bool {
        !self.levels.is_empty() && (opacity < u16::MAX || surface.has_alpha)
    }

    /// The screen area `surface` paints over completely: its shape, when it has no alpha
    /// channel and shows at full opacity.
    fn opaque(&self, surface: &Surface, opacity: u16) -> Vec<Rect> {
        if opacity < u16::MAX || surface.has_alpha {
            return Vec::new();
        }
        shape(surface)
            .filter_map(|rect| rect.intersect(self.screen()))
            .collect()
    }

    fn paint_background(&self, session: &Session, area: &[Rect]) -> Result<()> {
        if area.is_empty() {
            return Ok(());
        }
        let conn = &session.conn;
        let full = self.screen().x11()?;
        let area: Vec<_> = area.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &area)?;
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

    /// Paint `surface` at its opacity within `clip`, over what `beneath` says it shows.
    fn paint_surface(
        &mut self,
        session: &Session,
        (surface, opacity): (&Surface, u16),
        clip: &[Rect],
        beneath: Beneath,
    ) -> Result<()> {
        let conn = &session.conn;
        // A backdrop due to blur again is refreshed even where none of it shows, so that a
        // later frame never reuses a stale one.
        if let Beneath::Fresh { bounds, .. } = beneath {
            self.keep(surface.window, bounds)?;
        }
        let backdrop = match beneath {
            Beneath::Scene => None,
            Beneath::Kept(bounds) | Beneath::Fresh { bounds, .. } => self
                .backdrops
                .iter()
                .find(|kept| kept.window == surface.window && kept.area == bounds),
        };
        if let (Some(backdrop), Beneath::Fresh { .. }) = (backdrop, beneath) {
            self.blur(session, surface, backdrop)?;
        }
        if clip.is_empty() {
            return Ok(());
        }
        let clip: Vec<_> = clip.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
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
}

/// The surfaces a frame shows, bottom to top, with their opacity.
fn visible<'a>(scene: &'a Scene, config: &Config) -> Result<Vec<(&'a Surface, u16)>> {
    let now = Instant::now();
    let global = u16::try_from(u32::from(config.opacity) * u32::from(u16::MAX) / 100)?;
    Ok(scene
        .windows
        .iter()
        .filter_map(|surface| {
            let opacity = multiply_alpha(
                multiply_alpha(surface.opacity, global),
                surface.fade.sample(now),
            );
            (opacity > 0).then_some((surface, opacity))
        })
        .collect())
}

/// The screen area of `surface`'s shape, within its bounds.
fn shape(surface: &Surface) -> impl Iterator<Item = Rect> + '_ {
    let bounds = surface.bounds();
    let origin = (surface.geometry.x, surface.geometry.y);
    surface
        .shape
        .iter()
        .filter_map(move |rect| Rect::at(*rect, origin).intersect(bounds))
}
