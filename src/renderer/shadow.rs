use super::Renderer;
use crate::{
    animation::multiply_alpha,
    config::Config,
    picture::{Picture, Size},
    region::Rect,
    session::Session,
    surface::Surface,
};
use anyhow::{Context, Result};
use x11rb::{
    connection::Connection as _,
    protocol::{
        render::{Color, ConnectionExt as _, PictOp, Repeat},
        xproto::{ConnectionExt as _, CreateGCAux, ImageFormat, Rectangle, Window},
    },
};

/// A surface's shadow in a frame: black, darkest along the surface's edges and fading to
/// nothing `radius` pixels away. Its darkness at a pixel is the product of one profile along
/// each axis, which is what blurring a rectangle gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Shadow {
    /// Where it lies on the root: the surface's bounds, grown by the radius and moved by the
    /// offset.
    pub(super) rect: Rect,
    /// The size of the surface that casts it.
    pub(super) size: Size,
    pub(super) radius: u8,
    /// How dark it is where it is darkest.
    pub(super) strength: u16,
}

impl Shadow {
    /// The shadow `surface` casts at `opacity`: none while `shadow_radius` is zero, from a
    /// surface that casts none, or where X's 16-bit coordinates cannot address it.
    pub(super) fn cast(surface: &Surface, opacity: u16, config: &Config) -> Option<Self> {
        if config.shadow_radius == 0 || !surface.casts_shadow() {
            return None;
        }
        let darkest = u32::from(config.shadow_opacity) * u32::from(u16::MAX) / 100;
        let strength = multiply_alpha(opacity, u16::try_from(darkest).ok()?);
        let margin = i32::from(config.shadow_radius);
        let bounds = surface.bounds();
        let limit = i32::from(i16::MAX);
        if strength == 0
            || bounds.right - bounds.left + 2 * margin > limit
            || bounds.bottom - bounds.top + 2 * margin > limit
        {
            return None;
        }
        let (across, down) = (
            i32::from(config.shadow_offset_x),
            i32::from(config.shadow_offset_y),
        );
        let rect = Rect {
            left: bounds.left - margin + across,
            top: bounds.top - margin + down,
            right: bounds.right + margin + across,
            bottom: bounds.bottom + margin + down,
        };
        if rect.left < i32::from(i16::MIN)
            || rect.top < i32::from(i16::MIN)
            || rect.right > limit
            || rect.bottom > limit
        {
            return None;
        }
        Some(Self {
            rect,
            size: surface.size,
            radius: config.shadow_radius,
            strength,
        })
    }
}

/// A shadow's darkness along one axis, for a surface `length` pixels long: one value per
/// pixel from `radius` pixels before the surface to `radius` after it, 255 beneath the
/// surface away from its ends. Three box filters in a row blur the surface's extent, which
/// comes close to a Gaussian with integers alone, so both painters draw the same values.
pub(super) fn profile(length: u16, radius: u8) -> Vec<u8> {
    let margin = usize::from(radius);
    // Three widths that add up to a kernel reaching exactly `margin` pixels each way.
    let (each, rest) = ((2 * margin + 3) / 3, (2 * margin + 3) % 3);
    let widths = [
        each + usize::from(rest > 0),
        each + usize::from(rest > 1),
        each,
    ];
    let mut kernel = vec![1_u64];
    for width in widths {
        let mut spread = vec![0_u64; kernel.len() + width - 1];
        for (index, weight) in kernel.iter().enumerate() {
            for slot in spread.iter_mut().skip(index).take(width) {
                *slot += weight;
            }
        }
        kernel = spread;
    }
    // `sums[k]` adds up the first `k` weights.
    let sums: Vec<u64> = std::iter::once(0)
        .chain(kernel.iter().scan(0, |sum, weight| {
            *sum += weight;
            Some(*sum)
        }))
        .collect();
    let total = sums.last().copied().unwrap_or(1);
    let length = usize::from(length);
    (0..length + 2 * margin)
        .map(|position| {
            // The weights that land on the surface from this pixel.
            let first = (2 * margin).saturating_sub(position);
            let last = (2 * margin + length - position).min(kernel.len());
            let covered =
                sums.get(last).copied().unwrap_or(total) - sums.get(first).copied().unwrap_or(0);
            u8::try_from((covered * 255 + total / 2) / total).unwrap_or(u8::MAX)
        })
        .collect()
}

/// The pictures the `XRender` painter draws one surface's shadow from.
pub(super) struct Strips {
    pub(super) window: Window,
    /// The surface size and radius the strips hold profiles for.
    size: Size,
    radius: u8,
    /// How long a profile the strips have room for, across and down. Lengths round up, so
    /// that a resize uploads new profiles without allocating.
    room: (u16, u16),
    /// The profile along the x axis, one row that repeats down the shadow.
    across: Picture,
    /// The profile along the y axis, one column that repeats across the shadow.
    down: Picture,
    /// `down` at the strength of the frame being painted.
    faded: Picture,
}

impl Renderer {
    /// Paint `shadow`, which `window`'s surface casts, into the back buffer within `clip`.
    pub(super) fn paint_shadow(
        &mut self,
        session: &Session,
        window: Window,
        shadow: Shadow,
        clip: &[Rect],
    ) -> Result<()> {
        let Some(area) = shadow.rect.intersect(self.screen()) else {
            return Ok(());
        };
        if clip.is_empty() {
            return Ok(());
        }
        let (across, down, faded) = self.strips(session, window, shadow)?;
        let conn = &session.conn;
        let clip: Vec<_> = clip.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
        let pixel = Rectangle {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        let strength = Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: shadow.strength,
        };
        conn.render_fill_rectangles(PictOp::SRC, self.alpha.id, strength, &[pixel])?;
        let length = u16::try_from(shadow.rect.bottom - shadow.rect.top)?;
        conn.render_composite(
            PictOp::SRC,
            down,
            self.alpha.id,
            faded,
            0,
            0,
            0,
            0,
            0,
            0,
            1,
            length,
        )?;
        conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &clip)?;
        let target = area.x11()?;
        // An alpha-only source is black, so the two profiles and the strength multiply into
        // how much of it covers each pixel.
        conn.render_composite(
            PictOp::OVER,
            across,
            faded,
            self.back.id,
            i16::try_from(area.left - shadow.rect.left)?,
            0,
            0,
            i16::try_from(area.top - shadow.rect.top)?,
            target.x,
            target.y,
            target.width,
            target.height,
        )?;
        Ok(())
    }

    /// The strips for `shadow`, holding the profiles of its surface's size and radius. They
    /// are uploaded again when either changed, into the same pictures while those have room.
    fn strips(
        &mut self,
        session: &Session,
        window: Window,
        shadow: Shadow,
    ) -> Result<(u32, u32, u32)> {
        let index = self.shadows.iter().position(|kept| kept.window == window);
        if let Some(kept) = index.and_then(|index| self.shadows.get(index))
            && kept.size == shadow.size
            && kept.radius == shadow.radius
        {
            return Ok((kept.across.id, kept.down.id, kept.faded.id));
        }
        let across = profile(shadow.size.width, shadow.radius);
        let down = profile(shadow.size.height, shadow.radius);
        let (width, height) = (u16::try_from(across.len())?, u16::try_from(down.len())?);
        let fits = |kept: &Strips| kept.room.0 >= width && kept.room.1 >= height;
        if !index
            .and_then(|index| self.shadows.get(index))
            .is_some_and(fits)
        {
            let round = |length: u16| length.div_ceil(64).saturating_mul(64).max(length);
            let room = (round(width), round(height));
            let strips = Strips {
                window,
                size: shadow.size,
                radius: shadow.radius,
                room,
                across: self.strip(session, (room.0, 1))?,
                down: self.strip(session, (1, room.1))?,
                faded: self.strip(session, (1, room.1))?,
            };
            self.shadows.retain(|kept| kept.window != window);
            self.shadows.push(strips);
        }
        let kept = self
            .shadows
            .iter_mut()
            .find(|kept| kept.window == window)
            .context("a shadow has no strips")?;
        (kept.size, kept.radius) = (shadow.size, shadow.radius);
        upload(session, &kept.across, (width, 1), &across)?;
        upload(session, &kept.down, (1, height), &down)?;
        Ok((kept.across.id, kept.down.id, kept.faded.id))
    }

    /// A repeating alpha picture of `size`.
    fn strip(&self, session: &Session, size: (u16, u16)) -> Result<Picture> {
        let (width, height) = size;
        let layout = (Size { width, height }, self.a8);
        let picture = Picture::buffer(&session.conn, self.overlay, layout)?;
        picture.repeat(Repeat::NORMAL)?;
        Ok(picture)
    }
}

/// Write `values`, row by row, into the top left `size` of `picture`'s pixmap.
pub(super) fn upload(
    session: &Session,
    picture: &Picture,
    size: (u16, u16),
    values: &[u8],
) -> Result<()> {
    let conn = &session.conn;
    let (width, height) = size;
    let pixmap = picture.pixmap.context("a strip has no pixmap")?;
    // Each row of an image is padded to the server's scanline unit.
    let pad = conn
        .setup()
        .pixmap_formats
        .iter()
        .find(|format| format.depth == 8)
        .map(|format| usize::from(format.scanline_pad / 8))
        .context("the server has no 8-bit image format")?;
    let row = usize::from(width).div_ceil(pad) * pad;
    let mut image = Vec::with_capacity(row * usize::from(height));
    for line in values.chunks(usize::from(width)) {
        image.extend_from_slice(line);
        image.resize(image.len() + row - line.len(), 0);
    }
    let gc = conn.generate_id()?;
    conn.create_gc(gc, pixmap, &CreateGCAux::new())?;
    conn.put_image(
        ImageFormat::Z_PIXMAP,
        pixmap,
        gc,
        width,
        height,
        0,
        0,
        0,
        8,
        &image,
    )?;
    conn.free_gc(gc)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_symmetric_and_fade_over_the_radius() {
        // Given a surface longer than the shadow is wide, the profile is full beneath it,
        // about half at each end, and falls to nothing at the radius.
        let values = profile(100, 12);
        assert_eq!(values.len(), 124);
        let mirrored: Vec<_> = values.iter().rev().copied().collect();
        assert_eq!(values, mirrored);
        assert!(
            values
                .windows(2)
                .take(61)
                .all(|pair| matches!(pair, [a, b] if a <= b))
        );
        assert_eq!(values.first(), Some(&0));
        assert_eq!(values.get(62), Some(&255));
        // The last pixel outside the surface and the first beneath it straddle half.
        let (outside, beneath) = (values.get(11).copied(), values.get(12).copied());
        assert!(
            matches!((outside, beneath), (Some(110..=127), Some(128..=145))),
            "{values:?}"
        );
    }

    #[test]
    fn short_surfaces_never_reach_full_darkness() {
        // Given a surface shorter than the blur, even its middle is less than fully covered.
        let values = profile(4, 12);
        assert_eq!(values.len(), 28);
        let peak = values.iter().copied().max();
        assert!(matches!(peak, Some(40..=120)), "{values:?}");
        assert_eq!(profile(1, 1), [64, 128, 64]);
        assert_eq!(profile(10, 0), [255; 10]);
    }
}
