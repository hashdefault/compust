use super::{
    Renderer, Source, blur, corner,
    cover::{self, Layer},
    damage::{self, Blurred, Change, OUTPUT, Shown},
    shadow::Shadow,
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
        render::{Color, ConnectionExt as _, PictOp, Picture as PictureId},
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
    /// Its shadow, with where that shows in the repaint area: around the surface, never
    /// beneath it.
    pub(super) shadow: Option<(Shadow, Vec<Rect>)>,
    /// The radius of its rounded corners; zero when they are square.
    pub(super) corners: u8,
}

/// A surface a frame shows, with its opacity, the shadow it casts, and the radius of its
/// corners.
#[derive(Clone, Copy)]
struct Seen<'a> {
    surface: &'a Surface,
    opacity: u16,
    shadow: Option<Shadow>,
    corners: u8,
}

/// What a frame paints, which either painter carries out.
pub(super) struct Plan<'a> {
    /// Where the background shows in the repaint area.
    pub(super) background: Vec<Rect>,
    /// The surfaces, bottom to top, without those hidden whole along with their shadows.
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

    /// Whether one surface hides everything else, so that compositing changes nothing on
    /// the screen: the topmost surface shown, when it is still mapped, opaque, wholly
    /// rectangular, and covers the screen.
    pub(crate) fn covered(&self, scene: &Scene, config: &Config) -> Result<bool> {
        let visible = visible(scene, config)?;
        Ok(visible.last().is_some_and(|top| {
            let surface = top.surface;
            surface.mapped
                && top.opacity == u16::MAX
                && !surface.has_alpha
                && surface.rectangular()
                && top.corners == 0
                && surface.bounds().contains(self.screen())
        }))
    }

    /// Find what changed and plan the frame that shows it, with the area it repaints; `None`
    /// when nothing changed.
    fn plan<'a>(&mut self, visible: &[Seen<'a>]) -> Option<(Region, Plan<'a>)> {
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
            .map(|seen| {
                let window = seen.surface.window;
                let index = windows.iter().position(|kept| *kept == window);
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
            .map(|(seen, beneath)| Layer {
                opaque: self.opaque(seen),
                blur: match *beneath {
                    Beneath::Fresh { bounds, footprint } => Some((bounds, footprint)),
                    _ => None,
                },
            })
            .collect();
        let covers = cover::covers(&layers);
        let mut parts = Vec::with_capacity(visible.len());
        for (index, (seen, beneath)) in visible.iter().zip(beneath).enumerate() {
            let Seen {
                surface,
                opacity,
                corners,
                ..
            } = *seen;
            let in_sight = |clip: Vec<Rect>| match covers.above.get(index) {
                Some(cover) => cover.visible(clip),
                None => clip,
            };
            // A shadow lies around its surface, so what hides the surface can leave it in sight.
            let shadow = seen.shadow.map(|shadow| {
                let body = corner::interior(surface.bounds(), corners);
                let around = without(area.clip(std::iter::once(shadow.rect)), &body);
                (shadow, in_sight(around))
            });
            if covers.hidden.get(index) == Some(&true) {
                // Hidden whole, it need not blur, and its backdrop would go stale.
                self.backdrops.retain(|kept| kept.window != surface.window);
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.forget(surface.window);
                }
                if shadow.as_ref().is_some_and(|(_, clip)| !clip.is_empty()) {
                    parts.push(Part {
                        surface,
                        opacity,
                        clip: Vec::new(),
                        beneath: Beneath::Scene,
                        shadow,
                        corners,
                    });
                }
                continue;
            }
            parts.push(Part {
                surface,
                opacity,
                clip: in_sight(area.clip(shape(surface))),
                beneath,
                shadow,
                corners,
            });
        }
        let background = covers.background.visible(area.rects().to_vec());
        Some((area, Plan { background, parts }))
    }

    fn paint_xrender(&mut self, session: &Session, plan: &Plan<'_>) -> Result<()> {
        self.paint_background(session, &plan.background)?;
        self.shadows.retain(|kept| {
            plan.parts
                .iter()
                .any(|part| part.shadow.is_some() && part.surface.window == kept.window)
        });
        self.keep_disks(|radius| plan.parts.iter().any(|part| part.corners == radius));
        for part in &plan.parts {
            self.paint_surface(session, part)?;
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
    fn changes(&mut self, visible: &[Seen<'_>]) -> Vec<Change> {
        let shown: Vec<_> = visible
            .iter()
            .map(|seen| {
                let blur = self.blurs(seen.surface, seen.opacity);
                Shown::new(seen.surface, seen.opacity, blur, seen.shadow, seen.corners)
            })
            .collect();
        let mut changes = damage::changes(&self.shown, &shown);
        self.shown = shown;
        for (source, region) in std::mem::take(&mut self.pending) {
            let layer = match source {
                Source::Background => Some(0),
                Source::Window(window) => visible
                    .iter()
                    .position(|seen| seen.surface.window == window)
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
    fn blurred(&self, visible: &[Seen<'_>]) -> (Vec<Window>, Vec<Blurred>) {
        visible
            .iter()
            .enumerate()
            .filter(|(_, seen)| self.blurs(seen.surface, seen.opacity))
            .filter_map(|(index, seen)| {
                let surface = seen.surface;
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

    /// Whether a blurred backdrop shows through `surface`: when blur is on, a rule leaves it
    /// on, and the surface is translucent anywhere.
    fn blurs(&self, surface: &Surface, opacity: u16) -> bool {
        !self.levels.is_empty()
            && surface.overrides.blur.unwrap_or(true)
            && (opacity < u16::MAX || surface.has_alpha)
    }

    /// The screen area a surface paints over completely when it has no alpha channel and
    /// shows at full opacity: its shape, without the corners it rounds.
    fn opaque(&self, seen: &Seen<'_>) -> Vec<Rect> {
        let surface = seen.surface;
        if seen.opacity < u16::MAX || surface.has_alpha {
            return Vec::new();
        }
        let screen = self.screen();
        if seen.corners > 0 {
            return corner::interior(surface.bounds(), seen.corners)
                .into_iter()
                .filter_map(|rect| rect.intersect(screen))
                .collect();
        }
        shape(surface)
            .filter_map(|rect| rect.intersect(screen))
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

    /// Paint `part`: its surface at its opacity within its clip, over what it shows beneath
    /// it, and around it the shadow it casts.
    fn paint_surface(&mut self, session: &Session, part: &Part<'_>) -> Result<()> {
        let (surface, opacity, beneath) = (part.surface, part.opacity, part.beneath);
        let conn = &session.conn;
        // A backdrop due to blur again is refreshed even where none of it shows, so that a
        // later frame never reuses a stale one.
        if let Beneath::Fresh { bounds, .. } = beneath {
            self.keep(surface.window, bounds)?;
            let kept = self
                .backdrops
                .iter()
                .find(|kept| kept.window == surface.window && kept.area == bounds);
            if let Some(backdrop) = kept {
                self.blur(session, surface, backdrop)?;
            }
        }
        // The shadow follows the blur, which reads the scene beneath the surface without it.
        if let Some((shadow, around)) = &part.shadow {
            self.paint_shadow(session, surface.window, *shadow, around)?;
        }
        if part.clip.is_empty() {
            return Ok(());
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
        // Rounded corners draw through the disk times the opacity the alpha mask now holds,
        // and the backdrop's weights through the disk alone.
        let corners = part.corners;
        let rounded = if corners > 0 {
            let mask = self.corner_mask(session, corners, opacity)?;
            Some((mask, self.disk_picture(session, corners)?))
        } else {
            None
        };
        let backdrop = match beneath {
            Beneath::Scene => None,
            Beneath::Kept(bounds) | Beneath::Fresh { bounds, .. } => self
                .backdrops
                .iter()
                .find(|kept| kept.window == surface.window && kept.area == bounds),
        };
        let clip: Vec<_> = part
            .clip
            .iter()
            .map(|rect| rect.x11())
            .collect::<Result<_>>()?;
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &clip)?;
        if let Some(backdrop) = backdrop {
            let disk = rounded.map(|(_, disk)| (corners, disk));
            self.show_backdrop(session, surface, backdrop, &clip, disk)?;
        }
        match rounded {
            Some((mask, _)) => self.paint_rounded(session, part, mask),
            None => self.composite(session, surface),
        }
    }

    /// Composite all of `surface` over the back buffer through the alpha mask, within the
    /// clip the back buffer has.
    pub(super) fn composite(&self, session: &Session, surface: &Surface) -> Result<()> {
        session.conn.render_composite(
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

    /// Composite `backdrop` beneath `surface` within `clip`, as strongly as the surface covers
    /// each pixel: its alpha times the opacity the alpha mask holds. A transparent margin or
    /// shadow then shows little blur, and the blur fades in and out with the surface. With
    /// `rounded`, the radius of its corners and their disk, the weights take the disk's
    /// coverage at each corner, so blur never shows beyond the arcs.
    fn show_backdrop(
        &self,
        session: &Session,
        surface: &Surface,
        backdrop: &blur::Backdrop,
        clip: &[Rectangle],
        rounded: Option<(u8, PictureId)>,
    ) -> Result<()> {
        let conn = &session.conn;
        let area = backdrop.area.x11()?;
        let (mask, origin) = match &self.weights {
            Some(weights) if surface.has_alpha || rounded.is_some() => {
                conn.render_set_picture_clip_rectangles(weights.id, 0, 0, clip)?;
                conn.render_composite(
                    PictOp::SRC,
                    self.alpha.id,
                    surface.picture.id,
                    weights.id,
                    0,
                    0,
                    0,
                    0,
                    surface.geometry.x,
                    surface.geometry.y,
                    surface.size.width,
                    surface.size.height,
                )?;
                if let Some((radius, disk)) = rounded {
                    for (square, (across, down)) in corner::squares(surface.bounds(), radius) {
                        conn.render_composite(
                            PictOp::IN,
                            disk,
                            NONE,
                            weights.id,
                            i16::try_from(across)?,
                            i16::try_from(down)?,
                            0,
                            0,
                            i16::try_from(square.left)?,
                            i16::try_from(square.top)?,
                            u16::from(radius),
                            u16::from(radius),
                        )?;
                    }
                }
                (weights.id, (area.x, area.y))
            }
            _ => (self.alpha.id, (0, 0)),
        };
        conn.render_composite(
            PictOp::OVER,
            backdrop.picture.id,
            mask,
            self.back.id,
            0,
            0,
            origin.0,
            origin.1,
            area.x,
            area.y,
            area.width,
            area.height,
        )?;
        Ok(())
    }
}

/// `rects` without any part of `cut`.
fn without(rects: Vec<Rect>, cut: &[Rect]) -> Vec<Rect> {
    cut.iter().fold(rects, |rects, cut| {
        rects
            .into_iter()
            .flat_map(|rect| rect.minus(*cut))
            .collect()
    })
}

/// The surfaces a frame shows, bottom to top, with their opacity: their own, times their
/// rule's or else the global one, times their fade. Each casts the shadow `config` gives it,
/// and has its corners rounded as `config` says.
fn visible<'a>(scene: &'a Scene, config: &Config) -> Result<Vec<Seen<'a>>> {
    let now = Instant::now();
    let mut visible = Vec::with_capacity(scene.windows.len());
    for surface in &scene.windows {
        let percent = surface.overrides.opacity.unwrap_or(config.opacity);
        let configured = u16::try_from(u32::from(percent) * u32::from(u16::MAX) / 100)?;
        let opacity = multiply_alpha(
            multiply_alpha(surface.opacity, configured),
            surface.fade.sample(now),
        );
        if opacity > 0 {
            visible.push(Seen {
                surface,
                opacity,
                shadow: Shadow::cast(surface, opacity, config),
                corners: corner::radius(surface, config),
            });
        }
    }
    Ok(visible)
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
