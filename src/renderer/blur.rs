use super::Renderer;
use crate::{
    picture::{Format, Picture, Size},
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
    /// Replace the scene behind `surface`, within its shape, with a blurred copy. Each pass
    /// halves or doubles the resolution with bilinear sampling, which glamor-based servers
    /// keep on the GPU; they render convolution filters on the CPU instead.
    pub(super) fn blur(&self, session: &Session, surface: &Surface) -> Result<()> {
        let Some(bounds) = Bounds::new(surface, self.size, self.levels.len()) else {
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
            composite(conn, source, level.id, bounds.level(index + 1)?)?;
            source = level.id;
        }
        conn.render_set_picture_transform(self.back.id, scale(ONE))?;
        conn.render_set_picture_filter(self.back.id, b"nearest", &[])?;
        for (index, pair) in self.levels.windows(2).enumerate().rev() {
            if let [finer, coarser] = pair {
                conn.render_set_picture_transform(coarser.id, scale(ONE / 2))?;
                composite(conn, coarser.id, finer.id, bounds.level(index + 1)?)?;
            }
        }
        if let Some(finest) = self.levels.first() {
            conn.render_set_picture_clip_rectangles(
                self.back.id,
                surface.geometry.x,
                surface.geometry.y,
                &surface.shape,
            )?;
            conn.render_set_picture_transform(finest.id, scale(ONE / 2))?;
            composite(conn, finest.id, self.back.id, bounds.level(0)?)?;
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

/// The root area a blur reads and writes: the window plus enough margin that stale
/// pixels outside the area cannot reach it, aligned to the coarsest level.
#[derive(Debug, PartialEq, Eq)]
struct Bounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Bounds {
    fn new(surface: &Surface, root: Size, depth: usize) -> Option<Self> {
        Self::around(
            (
                i32::from(surface.geometry.x),
                i32::from(surface.geometry.y),
                i32::from(surface.size.width),
                i32::from(surface.size.height),
            ),
            root,
            depth,
        )
    }

    fn around(
        (x, y, width, height): (i32, i32, i32, i32),
        root: Size,
        depth: usize,
    ) -> Option<Self> {
        let (root_width, root_height) = (i32::from(root.width), i32::from(root.height));
        if x >= root_width || y >= root_height || x + width <= 0 || y + height <= 0 {
            return None;
        }
        let step = 1_i32 << depth;
        let margin = 2 * step;
        let align_up =
            |value: i32, limit: i32| ((value.min(limit) + step - 1) & !(step - 1)).min(limit);
        Some(Self {
            left: (x - margin).max(0) & !(step - 1),
            top: (y - margin).max(0) & !(step - 1),
            right: align_up(x + width + margin, root_width),
            bottom: align_up(y + height + margin, root_height),
        })
    }

    /// This area in the coordinates of pyramid `level`, where level 0 is the root.
    fn level(&self, level: usize) -> Result<Rectangle> {
        let scale = 1_i32 << level;
        let (left, top) = (self.left / scale, self.top / scale);
        Ok(Rectangle {
            x: i16::try_from(left)?,
            y: i16::try_from(top)?,
            width: u16::try_from((self.right + scale - 1) / scale - left)?,
            height: u16::try_from((self.bottom + scale - 1) / scale - top)?,
        })
    }
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

    fn corners(bounds: &Bounds, level: usize) -> Option<(i16, i16, u16, u16)> {
        let area = bounds.level(level).ok()?;
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
        let bounds = Bounds::around((101, 61, 100, 100), ROOT, 2);
        assert_eq!(
            bounds,
            Some(Bounds {
                left: 92,
                top: 52,
                right: 212,
                bottom: 172,
            })
        );
        assert_eq!(
            bounds.and_then(|bounds| corners(&bounds, 2)),
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
        let bounds = Bounds::around((-40, 650, 100, 100), size, 2);
        assert_eq!(
            bounds,
            Some(Bounds {
                left: 0,
                top: 640,
                right: 68,
                bottom: 701,
            })
        );
        assert_eq!(
            bounds.and_then(|bounds| corners(&bounds, 2)),
            Some((0, 160, 17, 16))
        );
        assert_eq!(Bounds::around((-500, 10, 100, 100), size, 2), None);
        assert_eq!(Bounds::around((1100, 10, 100, 100), size, 2), None);
    }
}
