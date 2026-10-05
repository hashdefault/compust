use super::{Renderer, paint::Part, shadow::upload};
use crate::{
    config::Config,
    picture::{Picture, Size},
    region::Rect,
    session::Session,
    surface::Surface,
};
use anyhow::Result;
use x11rb::protocol::render::{ConnectionExt as _, PictOp, Picture as PictureId};

/// The side of the widest disk: twice the largest radius the configuration accepts.
const WIDEST: u16 = 128;

/// The pictures the `XRender` painter rounds corners with.
#[derive(Default)]
pub(super) struct Masks {
    /// One disk per radius in use.
    disks: Vec<(u8, Picture)>,
    /// A disk times an opacity, for a surface shown below full opacity.
    scratch: Option<Picture>,
}

/// The radius `surface`'s corners are drawn with: what its rules give it, limited to half its
/// shorter side, border included, so that its arcs never overlap.
pub(super) fn radius(surface: &Surface, config: &Config) -> u8 {
    limit(surface.corner_radius(config), surface.size)
}

fn limit(radius: u8, size: Size) -> u8 {
    let half = size.width.min(size.height) / 2;
    u8::try_from(half).map_or(radius, |half| radius.min(half))
}

/// The squares at the corners of `bounds` that `radius` rounds, each with the pixel of the
/// disk on its top left: top left, top right, bottom left, then bottom right.
pub(super) fn squares(bounds: Rect, radius: u8) -> [(Rect, (i32, i32)); 4] {
    let r = i32::from(radius);
    let square = |left: i32, top: i32| Rect {
        left,
        top,
        right: left + r,
        bottom: top + r,
    };
    [
        (square(bounds.left, bounds.top), (0, 0)),
        (square(bounds.right - r, bounds.top), (r, 0)),
        (square(bounds.left, bounds.bottom - r), (0, r)),
        (square(bounds.right - r, bounds.bottom - r), (r, r)),
    ]
}

/// `bounds` without the squares `radius` rounds: a band across the middle and the bands
/// between the squares above and below it, leaving out those that are empty. Without a
/// radius, that is `bounds` itself.
pub(super) fn interior(bounds: Rect, radius: u8) -> Vec<Rect> {
    let r = i32::from(radius);
    [
        Rect {
            top: bounds.top + r,
            bottom: bounds.bottom - r,
            ..bounds
        },
        Rect {
            left: bounds.left + r,
            right: bounds.right - r,
            bottom: bounds.top + r,
            ..bounds
        },
        Rect {
            left: bounds.left + r,
            right: bounds.right - r,
            top: bounds.bottom - r,
            ..bounds
        },
    ]
    .into_iter()
    .filter(|rect| rect.right > rect.left && rect.bottom > rect.top)
    .collect()
}

/// The parts of `clip` within `areas`.
pub(super) fn within(clip: &[Rect], areas: &[Rect]) -> Vec<Rect> {
    clip.iter()
        .flat_map(|rect| areas.iter().filter_map(|area| rect.intersect(*area)))
        .collect()
}

impl Renderer {
    /// The mask that the corner squares of a surface are drawn through, at the opacity the
    /// alpha mask holds: the disk itself at full opacity, or else the disk times that
    /// opacity, written by one composite through the alpha mask.
    pub(super) fn corner_mask(
        &mut self,
        session: &Session,
        radius: u8,
        opacity: u16,
    ) -> Result<PictureId> {
        let disk = self.disk_picture(session, radius)?;
        if opacity == u16::MAX {
            return Ok(disk);
        }
        if self.corners.scratch.is_none() {
            let widest = Size {
                width: WIDEST,
                height: WIDEST,
            };
            let scratch = Picture::buffer(&session.conn, self.overlay, (widest, self.a8))?;
            self.corners.scratch = Some(scratch);
        }
        let scratch = self
            .corners
            .scratch
            .as_ref()
            .map_or(disk, |scratch| scratch.id);
        let side = 2 * u16::from(radius);
        session.conn.render_composite(
            PictOp::SRC,
            disk,
            self.alpha.id,
            scratch,
            0,
            0,
            0,
            0,
            0,
            0,
            side,
            side,
        )?;
        Ok(scratch)
    }

    /// The disk for `radius`, uploaded on its first use.
    pub(super) fn disk_picture(&mut self, session: &Session, radius: u8) -> Result<PictureId> {
        if let Some((_, kept)) = self.corners.disks.iter().find(|(kept, _)| *kept == radius) {
            return Ok(kept.id);
        }
        let side = 2 * u16::from(radius);
        let square = Size {
            width: side,
            height: side,
        };
        let picture = Picture::buffer(&session.conn, self.overlay, (square, self.a8))?;
        upload(session, &picture, (side, side), &disk(radius))?;
        let id = picture.id;
        self.corners.disks.push((radius, picture));
        Ok(id)
    }

    /// Draw `part`'s surface with its corners rounded: its interior through the alpha mask,
    /// and each corner square through `mask`, within the part's clip.
    pub(super) fn paint_rounded(
        &self,
        session: &Session,
        part: &Part<'_>,
        mask: PictureId,
    ) -> Result<()> {
        let conn = &session.conn;
        let (surface, radius) = (part.surface, part.corners);
        let bounds = surface.bounds();
        let inner = within(&part.clip, &interior(bounds, radius));
        if !inner.is_empty() {
            let inner: Vec<_> = inner.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
            conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &inner)?;
            self.composite(session, surface)?;
        }
        let squares = squares(bounds, radius);
        let outer = within(&part.clip, &squares.map(|(square, _)| square));
        if outer.is_empty() {
            return Ok(());
        }
        let shown: Vec<_> = outer.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &shown)?;
        for (square, (across, down)) in squares {
            if !outer.iter().any(|rect| rect.intersect(square).is_some()) {
                continue;
            }
            conn.render_composite(
                PictOp::OVER,
                surface.picture.id,
                mask,
                self.back.id,
                i16::try_from(square.left - bounds.left)?,
                i16::try_from(square.top - bounds.top)?,
                i16::try_from(across)?,
                i16::try_from(down)?,
                i16::try_from(square.left)?,
                i16::try_from(square.top)?,
                u16::from(radius),
                u16::from(radius),
            )?;
        }
        Ok(())
    }

    /// Release the disks of radii for which `used` is false.
    pub(super) fn keep_disks(&mut self, used: impl Fn(u8) -> bool) {
        self.corners.disks.retain(|(radius, _)| used(*radius));
    }
}

/// A disk of `radius` in a square twice as wide, row by row: how much of each pixel lies
/// inside the circle, from 0 to 255. Each quadrant is one corner of a window, so both
/// painters draw every corner from these values. A pixel counts how many of 16 × 16 points
/// spread evenly across it lie inside, with integers alone.
pub(super) fn disk(radius: u8) -> Vec<u8> {
    const SAMPLES: i32 = 16;
    // Coordinates count halves of the space between points, so that every point, in the
    // middle of its share of a pixel, has whole ones.
    let unit = 2 * SAMPLES;
    let center = unit * i32::from(radius);
    let side = 2 * i32::from(radius);
    let inside = |x: i32, y: i32| {
        let (across, down) = (x - center, y - center);
        across * across + down * down <= center * center
    };
    let total = SAMPLES * SAMPLES;
    (0..side)
        .flat_map(|row| (0..side).map(move |column| (column, row)))
        .map(|(column, row)| {
            let count: i32 = (0..SAMPLES)
                .flat_map(|y| (0..SAMPLES).map(move |x| (x, y)))
                .map(|(x, y)| i32::from(inside(unit * column + 2 * x + 1, unit * row + 2 * y + 1)))
                .sum();
            u8::try_from((count * 255 + total / 2) / total).unwrap_or(u8::MAX)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disks_are_symmetric_and_fill_toward_their_middle() {
        // Given radii from one pixel up, each disk mirrors across both axes and its diagonal,
        // and within a corner never thins toward the middle.
        for radius in [1_u8, 2, 3, 4, 8, 13, 64] {
            let side = 2 * usize::from(radius);
            let values = disk(radius);
            assert_eq!(values.len(), side * side);
            let at = |x: usize, y: usize| values.get(y * side + x).copied().unwrap();
            let half = usize::from(radius);
            for y in 0..side {
                for x in 0..side {
                    let value = at(x, y);
                    assert_eq!(value, at(side - 1 - x, y), "radius {radius} at {x},{y}");
                    assert_eq!(value, at(x, side - 1 - y), "radius {radius} at {x},{y}");
                    assert_eq!(value, at(y, x), "radius {radius} at {x},{y}");
                    if x + 1 < half && y < half {
                        assert!(value <= at(x + 1, y), "radius {radius} at {x},{y}");
                    }
                    if y + 1 < half && x < half {
                        assert!(value <= at(x, y + 1), "radius {radius} at {x},{y}");
                    }
                }
            }
            // The pixels at the middle lie wholly inside from radius 2, and the corner pixels
            // wholly outside from radius 4, where the circle passes their inner corners.
            if radius >= 2 {
                assert_eq!(at(half - 1, half - 1), 255, "radius {radius}");
            }
            if radius >= 4 {
                assert_eq!(at(0, 0), 0, "radius {radius}");
            }
        }
        assert!(disk(0).is_empty());
        assert_eq!(disk(1), [202; 4]);
        assert_eq!(
            disk(8).get(..16),
            Some(&[0, 0, 0, 0, 48, 151, 218, 250, 250, 218, 151, 48, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn a_disk_covers_the_area_of_its_circle() {
        // Given the area a circle covers, the disk's values add up to it within a small
        // fraction of the pixels its edge crosses.
        for radius in [8_u8, 13, 64] {
            let covered: f64 = disk(radius)
                .iter()
                .map(|&value| f64::from(value) / 255.0)
                .sum();
            let r = f64::from(radius);
            let area = std::f64::consts::PI * r * r;
            assert!(
                (covered - area).abs() < r / 16.0,
                "radius {radius}: {covered} against {area}"
            );
        }
    }

    #[test]
    fn corner_squares_and_the_interior_tile_the_bounds() {
        // Given bounds and a radius, the four squares and the interior cover every pixel once.
        let bounds = Rect::new(10, 20, 30, 16);
        for radius in [0_u8, 1, 4, 8] {
            let mut pieces = interior(bounds, radius);
            pieces.extend(
                squares(bounds, radius)
                    .into_iter()
                    .map(|(square, _)| square)
                    .filter(|square| square.right > square.left),
            );
            for y in bounds.top - 1..=bounds.bottom {
                for x in bounds.left - 1..=bounds.right {
                    let inside = (bounds.left..bounds.right).contains(&x)
                        && (bounds.top..bounds.bottom).contains(&y);
                    let count = pieces
                        .iter()
                        .filter(|piece| {
                            (piece.left..piece.right).contains(&x)
                                && (piece.top..piece.bottom).contains(&y)
                        })
                        .count();
                    assert_eq!(count, usize::from(inside), "radius {radius} at {x},{y}");
                }
            }
        }
        assert_eq!(interior(bounds, 0), [bounds]);
        let [top_left, top_right, bottom_left, bottom_right] = squares(bounds, 8);
        assert_eq!(top_left, (Rect::new(10, 20, 8, 8), (0, 0)));
        assert_eq!(top_right, (Rect::new(32, 20, 8, 8), (8, 0)));
        assert_eq!(bottom_left, (Rect::new(10, 28, 8, 8), (0, 8)));
        assert_eq!(bottom_right, (Rect::new(32, 28, 8, 8), (8, 8)));
        // A square window of twice the radius is all corners.
        assert!(interior(Rect::new(0, 0, 16, 16), 8).is_empty());
    }

    #[test]
    fn radii_stop_at_half_the_shorter_side() {
        let size = |width, height| Size { width, height };
        assert_eq!(limit(8, size(100, 50)), 8);
        assert_eq!(limit(64, size(100, 50)), 25);
        assert_eq!(limit(8, size(15, 300)), 7);
        assert_eq!(limit(8, size(1, 1)), 0);
        assert_eq!(limit(0, size(1000, 1000)), 0);
        assert_eq!(limit(64, size(u16::MAX, u16::MAX)), 64);
    }
}
