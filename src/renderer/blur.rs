use super::Renderer;
use crate::{
    picture::{Format, Picture, Size},
    region::Rect,
    session::Session,
    surface::Surface,
};
use anyhow::Result;
use std::rc::Rc;
use x11rb::{
    NONE,
    protocol::{
        render::{ConnectionExt as _, Fixed, PictOp, Picture as PictureId, Repeat, Transform},
        xproto::{Rectangle, Window},
    },
    rust_connection::RustConnection,
};

const ONE: Fixed = 1 << 16;

impl Renderer {
    /// Replace the scene behind `surface`, within `clip`, with a blurred copy. The scene must
    /// be current across the surface's whole footprint. Each pass halves or doubles the
    /// resolution with bilinear sampling, which glamor-based servers keep on the GPU; they
    /// render convolution filters on the CPU instead.
    pub(super) fn blur(
        &self,
        session: &Session,
        surface: &Surface,
        clip: &[Rectangle],
    ) -> Result<()> {
        let Some(bounds) = footprint(surface, self.size, self.levels.len()) else {
            return Ok(());
        };
        let conn = &session.conn;
        let full = Rectangle {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        };
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &[full])?;
        conn.render_set_picture_filter(self.back.id, b"bilinear", &[])?;
        let mut source = self.back.id;
        for (index, level) in self.levels.iter().enumerate() {
            conn.render_set_picture_transform(source, scale(2 * ONE))?;
            composite(conn, source, level.id, level_area(bounds, index + 1)?)?;
            source = level.id;
        }
        conn.render_set_picture_transform(self.back.id, scale(ONE))?;
        conn.render_set_picture_filter(self.back.id, b"nearest", &[])?;
        for (index, pair) in self.levels.windows(2).enumerate().rev() {
            if let [finer, coarser] = pair {
                conn.render_set_picture_transform(coarser.id, scale(ONE / 2))?;
                composite(conn, coarser.id, finer.id, level_area(bounds, index + 1)?)?;
            }
        }
        if let Some(finest) = self.levels.first() {
            conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, clip)?;
            conn.render_set_picture_transform(finest.id, scale(ONE / 2))?;
            composite(conn, finest.id, self.back.id, level_area(bounds, 0)?)?;
        }
        Ok(())
    }
}

/// Pyramid depth for a configured radius; the blur spans about 2^depth pixels, so radii
/// round to the nearest power of two.
pub(super) fn depth(radius: u8) -> usize {
    if radius == 0 {
        return 0;
    }
    (1..=4)
        .find(|depth| 2 * u32::from(radius) < 3 << depth)
        .unwrap_or(4)
}

/// Allocate one buffer per pyramid level, each half the size of the previous one.
pub(super) fn pyramid(
    conn: &Rc<RustConnection>,
    root: Window,
    (size, format): (Size, Format),
    depth: usize,
) -> Result<Vec<Picture>> {
    let mut levels = Vec::with_capacity(depth);
    let mut level = size;
    for _ in 0..depth {
        level = Size {
            width: level.width.div_ceil(2),
            height: level.height.div_ceil(2),
        };
        let picture = Picture::buffer(conn, root, (level, format))?;
        picture.repeat(Repeat::PAD)?;
        conn.render_set_picture_filter(picture.id, b"bilinear", &[])?;
        levels.push(picture);
    }
    Ok(levels)
}

/// The root area a blur of `surface` reads and writes: the window plus enough margin that
/// stale pixels outside the area cannot reach it, aligned to the coarsest level. Changing
/// anything inside it can change the blurred pixels.
pub(super) fn footprint(surface: &Surface, root: Size, depth: usize) -> Option<Rect> {
    around(surface.bounds(), root, depth)
}

fn around(window: Rect, root: Size, depth: usize) -> Option<Rect> {
    let (root_width, root_height) = (i32::from(root.width), i32::from(root.height));
    if window.left >= root_width
        || window.top >= root_height
        || window.right <= 0
        || window.bottom <= 0
    {
        return None;
    }
    let step = 1_i32 << depth;
    let margin = 2 * step;
    let align_up =
        |value: i32, limit: i32| ((value.min(limit) + step - 1) & !(step - 1)).min(limit);
    Some(Rect {
        left: (window.left - margin).max(0) & !(step - 1),
        top: (window.top - margin).max(0) & !(step - 1),
        right: align_up(window.right + margin, root_width),
        bottom: align_up(window.bottom + margin, root_height),
    })
}

/// `bounds` in the coordinates of pyramid `level`, where level 0 is the root.
fn level_area(bounds: Rect, level: usize) -> Result<Rectangle> {
    let scale = 1_i32 << level;
    let (left, top) = (bounds.left / scale, bounds.top / scale);
    Ok(Rectangle {
        x: i16::try_from(left)?,
        y: i16::try_from(top)?,
        width: u16::try_from((bounds.right + scale - 1) / scale - left)?,
        height: u16::try_from((bounds.bottom + scale - 1) / scale - top)?,
    })
}

fn scale(factor: Fixed) -> Transform {
    Transform {
        matrix11: factor,
        matrix12: 0,
        matrix13: 0,
        matrix21: 0,
        matrix22: factor,
        matrix23: 0,
        matrix31: 0,
        matrix32: 0,
        matrix33: ONE,
    }
}

/// Copy `area` of `source`, sampled through its transform, to the same coordinates of `target`.
fn composite(
    conn: &RustConnection,
    source: PictureId,
    target: PictureId,
    area: Rectangle,
) -> Result<()> {
    conn.render_composite(
        PictOp::SRC,
        source,
        NONE,
        target,
        area.x,
        area.y,
        0,
        0,
        area.x,
        area.y,
        area.width,
        area.height,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: Size = Size {
        width: 1920,
        height: 1080,
    };

    fn corners(bounds: Rect, level: usize) -> Option<(i16, i16, u16, u16)> {
        let area = level_area(bounds, level).ok()?;
        Some((area.x, area.y, area.width, area.height))
    }

    #[test]
    fn radii_round_to_the_nearest_power_of_two() {
        let depths: Vec<_> = (0..=16).map(depth).collect();
        assert_eq!(depths, [0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4]);
    }

    #[test]
    fn bounds_add_margin_and_align_to_the_coarsest_level() {
        // Given a window away from the edges, the area grows by twice the level step.
        let bounds = around(Rect::new(101, 61, 100, 100), ROOT, 2);
        assert_eq!(
            bounds,
            Some(Rect {
                left: 92,
                top: 52,
                right: 212,
                bottom: 172,
            })
        );
        assert_eq!(
            bounds.and_then(|bounds| corners(bounds, 2)),
            Some((23, 13, 30, 30))
        );
    }

    #[test]
    fn bounds_stay_on_the_root_and_cover_odd_edges() {
        // Given windows past each edge, the area clips to the root; odd sizes round up.
        let size = Size {
            width: 1001,
            height: 701,
        };
        let bounds = around(Rect::new(-40, 650, 100, 100), size, 2);
        assert_eq!(
            bounds,
            Some(Rect {
                left: 0,
                top: 640,
                right: 68,
                bottom: 701,
            })
        );
        assert_eq!(
            bounds.and_then(|bounds| corners(bounds, 2)),
            Some((0, 160, 17, 16))
        );
        assert_eq!(around(Rect::new(-500, 10, 100, 100), size, 2), None);
        assert_eq!(around(Rect::new(1100, 10, 100, 100), size, 2), None);
    }
}
