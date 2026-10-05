use super::{Renderer, paint::Part, shadow::upload};
use crate::{
    config::Config,
    picture::{Picture, Size},
    region::Rect,
    session::Session,
    surface::Surface,
};
use anyhow::Result;
use x11rb::protocol::render::{Color, ConnectionExt as _, PictOp, Picture as PictureId, Repeat};

/// The side of the widest disk: twice the largest radius the configuration accepts.
const WIDEST: u16 = 128;

/// The pictures the `XRender` painter rounds corners with.
#[derive(Default)]
pub(super) struct Masks {
    /// One disk per radius in use.
    disks: Vec<(u8, Picture)>,
    /// A disk times an opacity, for a surface shown below full opacity.
    scratch: Option<Picture>,
    /// The content and border coverage of each radius and border width in use; see `bands`.
    bands: Vec<((u8, u8), Picture, Picture)>,
    /// The color of the border being drawn, one repeating ARGB pixel.
    color: Option<Picture>,
    /// The four corner squares of a bordered surface, composed before they are shown.
    composed: Option<Picture>,
}

/// The pictures that draw the corners of a surface whose border follows its arcs.
pub(super) struct Bordered {
    /// How much of each pixel the content's arc covers, in the disk's layout.
    pub(super) inner: PictureId,
    /// How much lies between the content's arc and the outer one, where the border shows.
    pub(super) ring: PictureId,
    pub(super) color: PictureId,
    pub(super) composed: PictureId,
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

/// The width of the X border that the arcs of corners of `radius` carry around with them.
pub(super) fn border(surface: &Surface, radius: u8) -> Option<u8> {
    let width = u8::try_from(surface.geometry.border_width).unwrap_or(radius);
    (radius > 0 && width > 0).then_some(width.min(radius))
}

/// For corners of `radius` around a border `border` pixels wide, in the disk's layout: how
/// much of each pixel lies inside the content's arc, of radius `radius - border` on the same
/// center, and how much lies between that arc and the outer one, where the border follows
/// the corner. The two add up to the disk of `radius`. A border at least as wide as the radius
/// leaves no content in the corner square.
pub(super) fn bands(radius: u8, border: u8) -> (Vec<u8>, Vec<u8>) {
    let outer = disk(radius);
    let content = disk(radius.saturating_sub(border));
    let (side, inner_side, shift) = (
        2 * usize::from(radius),
        2 * usize::from(radius.saturating_sub(border)),
        usize::from(border),
    );
    let inner: Vec<u8> = (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (Some(column), Some(row)) = (x.checked_sub(shift), y.checked_sub(shift)) else {
                return 0;
            };
            if column >= inner_side || row >= inner_side {
                return 0;
            }
            content.get(row * inner_side + column).copied().unwrap_or(0)
        })
        .collect();
    let ring = outer
        .iter()
        .zip(&inner)
        .map(|(outer, inner)| outer.saturating_sub(*inner))
        .collect();
    (inner, ring)
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
        bordered: Option<&Bordered>,
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
        let visible: Vec<_> = squares
            .into_iter()
            .filter(|(square, _)| outer.iter().any(|rect| rect.intersect(*square).is_some()))
            .collect();
        if let Some(bordered) = bordered {
            return self.paint_bordered(session, surface, radius, bordered, &visible);
        }
        for (square, (across, down)) in visible {
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

    /// The pictures for the corners of a surface with corners of `radius` and a border
    /// `border` pixels wide that follows them, made on first use; `None` where the server
    /// has no ARGB format, which leaves the outer arc alone.
    pub(super) fn bordered(
        &mut self,
        session: &Session,
        radius: u8,
        border: u8,
    ) -> Result<Option<Bordered>> {
        let Some(argb) = self.argb else {
            return Ok(None);
        };
        let side = 2 * u16::from(radius);
        let square = Size {
            width: side,
            height: side,
        };
        if !self
            .corners
            .bands
            .iter()
            .any(|(kept, _, _)| *kept == (radius, border))
        {
            let (inner, ring) = bands(radius, border);
            let inner_picture = Picture::buffer(&session.conn, self.overlay, (square, self.a8))?;
            upload(session, &inner_picture, (side, side), &inner)?;
            let ring_picture = Picture::buffer(&session.conn, self.overlay, (square, self.a8))?;
            upload(session, &ring_picture, (side, side), &ring)?;
            self.corners
                .bands
                .push(((radius, border), inner_picture, ring_picture));
        }
        if self.corners.color.is_none() {
            let pixel = Size {
                width: 1,
                height: 1,
            };
            let color = Picture::buffer(&session.conn, self.overlay, (pixel, argb))?;
            color.repeat(Repeat::NORMAL)?;
            self.corners.color = Some(color);
        }
        if self.corners.composed.is_none() {
            let widest = Size {
                width: WIDEST,
                height: WIDEST,
            };
            let composed = Picture::buffer(&session.conn, self.overlay, (widest, argb))?;
            self.corners.composed = Some(composed);
        }
        let masks = &self.corners;
        let found = masks
            .bands
            .iter()
            .find(|(kept, _, _)| *kept == (radius, border));
        Ok(match (found, &masks.color, &masks.composed) {
            (Some((_, inner, ring)), Some(color), Some(composed)) => Some(Bordered {
                inner: inner.id,
                ring: ring.id,
                color: color.id,
                composed: composed.id,
            }),
            _ => None,
        })
    }

    /// Draw the corner squares `shown` of `surface`, whose border follows its arcs, as
    /// `bordered` holds them. In a scratch picture laid out as the disk is, each square takes
    /// the surface's pixels inside the content's arc, and the border's color, taken from the
    /// surface's top-left pixel, which the border always covers, between the two arcs. The
    /// squares then show through the alpha mask's opacity.
    fn paint_bordered(
        &self,
        session: &Session,
        surface: &Surface,
        radius: u8,
        bordered: &Bordered,
        shown: &[(Rect, (i32, i32))],
    ) -> Result<()> {
        let conn = &session.conn;
        let bounds = surface.bounds();
        let size = u16::from(radius);
        conn.render_fill_rectangles(
            PictOp::SRC,
            bordered.composed,
            Color {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            },
            &[x11rb::protocol::xproto::Rectangle {
                x: 0,
                y: 0,
                width: 2 * size,
                height: 2 * size,
            }],
        )?;
        conn.render_composite(
            PictOp::SRC,
            surface.picture.id,
            x11rb::NONE,
            bordered.color,
            0,
            0,
            0,
            0,
            0,
            0,
            1,
            1,
        )?;
        for (square, (across, down)) in shown {
            let (across, down) = (i16::try_from(*across)?, i16::try_from(*down)?);
            conn.render_composite(
                PictOp::SRC,
                surface.picture.id,
                bordered.inner,
                bordered.composed,
                i16::try_from(square.left - bounds.left)?,
                i16::try_from(square.top - bounds.top)?,
                across,
                down,
                across,
                down,
                size,
                size,
            )?;
        }
        conn.render_composite(
            PictOp::ADD,
            bordered.color,
            bordered.ring,
            bordered.composed,
            0,
            0,
            0,
            0,
            0,
            0,
            2 * size,
            2 * size,
        )?;
        for (square, (across, down)) in shown {
            conn.render_composite(
                PictOp::OVER,
                bordered.composed,
                self.alpha.id,
                self.back.id,
                i16::try_from(*across)?,
                i16::try_from(*down)?,
                0,
                0,
                i16::try_from(square.left)?,
                i16::try_from(square.top)?,
                size,
                size,
            )?;
        }
        Ok(())
    }

    /// Release the disks of radii for which `used` is false.
    pub(super) fn keep_disks(&mut self, used: impl Fn(u8) -> bool) {
        self.corners.disks.retain(|(radius, _)| used(*radius));
    }

    pub(super) fn keep_bands(&mut self, used: impl Fn(u8, u8) -> bool) {
        self.corners
            .bands
            .retain(|((radius, border), _, _)| used(*radius, *border));
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
    fn border_bands_partition_each_outer_disk() {
        for (radius, border) in [(15_u8, 2_u8), (20, 4), (8, 8)] {
            let outer = disk(radius);
            let (inner, ring) = bands(radius, border);
            assert_eq!(outer.len(), inner.len());
            assert_eq!(outer.len(), ring.len());
            for ((outer, inner), ring) in outer.iter().zip(&inner).zip(&ring) {
                assert!(*inner <= *outer);
                assert_eq!(u16::from(*inner) + u16::from(*ring), u16::from(*outer));
            }
            if radius == border {
                assert!(inner.iter().all(|coverage| *coverage == 0));
                assert_eq!(ring, outer);
            }
        }
        let outer = disk(15);
        let (inner, ring) = bands(15, 0);
        assert_eq!(inner, outer);
        assert!(ring.iter().all(|coverage| *coverage == 0));
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
