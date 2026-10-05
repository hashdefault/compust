use super::{Renderer, corner::within};
use crate::{
    animation::multiply_alpha,
    config::Config,
    picture::{Picture, Size},
    region::{Rect, without},
    session::Session,
    surface::Surface,
};
use anyhow::{Context, Result};
use x11rb::{
    connection::Connection as _,
    protocol::{
        render::{Color, ConnectionExt as _, PictOp, Picture as PictureId, Repeat},
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
    /// The radius of the surface's rounded corners, which round the shadow too.
    pub(super) corners: u8,
    /// How far the shadow lies from its surface, right and down.
    pub(super) offset: (i8, i8),
}

impl Shadow {
    /// The shadow `surface` casts at `opacity`, with corners of `corners`: none while
    /// `shadow_radius` is zero, from a surface that casts none, or where X's 16-bit
    /// coordinates cannot address it.
    pub(super) fn cast(
        surface: &Surface,
        opacity: u16,
        config: &Config,
        corners: u8,
    ) -> Option<Self> {
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
            corners,
            offset: (config.shadow_offset_x, config.shadow_offset_y),
        })
    }
}

/// A shadow's darkness along one axis, for a surface `length` pixels long: one value per
/// pixel from `radius` pixels before the surface to `radius` after it, 255 beneath the
/// surface away from its ends. Three box filters in a row blur the surface's extent, which
/// comes close to a Gaussian with integers alone, so both painters draw the same values.
pub(super) fn profile(length: u16, radius: u8) -> Vec<u8> {
    let margin = usize::from(radius);
    let kernel = kernel(radius);
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

/// Three box filters in a row, with widths that add up to a kernel reaching exactly `radius`
/// pixels each way: `2 × radius + 1` integer weights.
fn kernel(radius: u8) -> Vec<u64> {
    let margin = usize::from(radius);
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
    kernel
}

/// How much the blur of the part that `corners` cuts off a surface's top-left corner darkens
/// each pixel of the square `corners + 2 × radius` wide at the top left of its shadow, row by
/// row, from 0 to 255. Rounding a surface removes that part, so its shadow is the product of
/// the profiles less this near each corner; the other corners mirror it. Both blurs separate
/// into one pass along each axis, with the kernel of the profiles.
pub(super) fn correction(corners: u8, radius: u8) -> Vec<u8> {
    let kernel = kernel(radius);
    let total = kernel.iter().sum::<u64>().max(1);
    let (r, two) = (usize::from(corners), 2 * usize::from(radius));
    let side = r + two;
    let disk = super::corner::disk(corners);
    // How much of each pixel of the corner square the rounding cuts off.
    let cut = |u: usize, v: usize| 255 - u64::from(disk.get(v * 2 * r + u).copied().unwrap_or(255));
    // Pixel `x` of the shadow takes weight `k` from surface pixel `x + k - 2 × radius`, as the
    // profiles do.
    let source = |x: usize, k: usize| (x + k).checked_sub(two).filter(|&u| u < r);
    let across: Vec<Vec<u64>> = (0..r)
        .map(|v| {
            (0..side)
                .map(|x| {
                    kernel
                        .iter()
                        .enumerate()
                        .filter_map(|(k, weight)| Some(weight * cut(source(x, k)?, v)))
                        .sum()
                })
                .collect()
        })
        .collect();
    (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .map(|(x, y)| {
            let blurred: u64 = kernel
                .iter()
                .enumerate()
                .filter_map(|(l, weight)| {
                    let row = across.get(source(y, l)?)?;
                    Some(weight * row.get(x)?)
                })
                .sum();
            let square = total * total;
            u8::try_from((blurred + square / 2) / square).unwrap_or(u8::MAX)
        })
        .collect()
}

/// Where the shadow of a rounded surface differs from the product of its profiles: patches of
/// values to draw instead, from 0 to 255, each with its rectangle relative to the shadow's
/// top left. `correction` is that of `shadow`'s corners and radius. Near each corner of the
/// shadow, the blur of the cut-off corner is subtracted. Within each of the surface's corner
/// squares, the shadow shows as much as the surface leaves each pixel uncovered, so that no
/// background shows between the two and a translucent surface is no darker for its shadow;
/// beneath the rest of the surface it is never drawn. Patches that would overlap are merged.
pub(super) fn patches(shadow: &Shadow, correction: &[u8]) -> Vec<(Rect, Vec<u8>)> {
    if shadow.corners == 0 {
        return Vec::new();
    }
    let across = profile(shadow.size.width, shadow.radius);
    let down = profile(shadow.size.height, shadow.radius);
    let (whole, body) = layout(shadow);
    let (corners, width, height) = (
        i32::from(shadow.corners),
        body.right - body.left,
        body.bottom - body.top,
    );
    let side = usize::from(shadow.corners) + 2 * usize::from(shadow.radius);
    let disk = super::corner::disk(shadow.corners);
    // Each corner's correction, mirrored toward its corner of the shadow.
    let corrected = |x: i32, y: i32| -> u64 {
        [
            (x, y),
            (whole.right - 1 - x, y),
            (x, whole.bottom - 1 - y),
            (whole.right - 1 - x, whole.bottom - 1 - y),
        ]
        .into_iter()
        .filter_map(|(cx, cy)| {
            let (cx, cy) = (usize::try_from(cx).ok()?, usize::try_from(cy).ok()?);
            (cx < side && cy < side)
                .then(|| u64::from(correction.get(cy * side + cx).copied().unwrap_or(0)))
        })
        .sum()
    };
    let profile_at = |values: &[u8], at: i32| {
        usize::try_from(at)
            .ok()
            .and_then(|at| values.get(at))
            .map_or(0, |value| u64::from(*value))
    };
    let value = |x: i32, y: i32| -> u8 {
        let product = (profile_at(&across, x) * profile_at(&down, y) + 127) / 255;
        let mut shown = product.saturating_sub(corrected(x, y));
        let (sx, sy) = (x - body.left, y - body.top);
        if (0..width).contains(&sx) && (0..height).contains(&sy) {
            let (cx, cy) = (sx.min(width - 1 - sx), sy.min(height - 1 - sy));
            shown = if cx < corners && cy < corners {
                let at = usize::try_from(cy * 2 * corners + cx).unwrap_or(usize::MAX);
                let covered = u64::from(disk.get(at).copied().unwrap_or(255));
                (shown * (255 - covered) + 127) / 255
            } else {
                0
            };
        }
        u8::try_from(shown.min(255)).unwrap_or(u8::MAX)
    };
    areas(shadow, whole, body)
        .into_iter()
        .map(|rect| {
            let values = (rect.top..rect.bottom)
                .flat_map(|y| (rect.left..rect.right).map(move |x| (x, y)))
                .map(|(x, y)| value(x, y))
                .collect();
            (rect, values)
        })
        .collect()
}

/// The whole of `shadow` and its surface, relative to the shadow's top left.
fn layout(shadow: &Shadow) -> (Rect, Rect) {
    let margin = i32::from(shadow.radius);
    let (width, height) = (i32::from(shadow.size.width), i32::from(shadow.size.height));
    let whole = Rect {
        left: 0,
        top: 0,
        right: width + 2 * margin,
        bottom: height + 2 * margin,
    };
    let (left, top) = (
        margin - i32::from(shadow.offset.0),
        margin - i32::from(shadow.offset.1),
    );
    let body = Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    (whole, body)
}

/// Where the patches of `shadow` lie within `whole`: at each corner, the square its
/// correction reaches together with the surface's corner square there, merged where they
/// would overlap so that each pixel is drawn once.
fn areas(shadow: &Shadow, whole: Rect, body: Rect) -> Vec<Rect> {
    let side = i32::from(shadow.corners) + 2 * i32::from(shadow.radius);
    let tile = |left: i32, top: i32| Rect {
        left,
        top,
        right: left + side,
        bottom: top + side,
    };
    let tiles = [
        tile(0, 0),
        tile(whole.right - side, 0),
        tile(0, whole.bottom - side),
        tile(whole.right - side, whole.bottom - side),
    ];
    let mut rects: Vec<Rect> = tiles
        .into_iter()
        .zip(super::corner::squares(body, shadow.corners))
        .filter_map(|(tile, (gap, _))| {
            let hull = gap.intersect(whole).map_or(tile, |gap| tile.hull(gap));
            hull.intersect(whole)
        })
        .collect();
    while let Some((first, second)) = (0..rects.len())
        .flat_map(|i| (i + 1..rects.len()).map(move |j| (i, j)))
        .find(|&(i, j)| match (rects.get(i), rects.get(j)) {
            (Some(a), Some(b)) => a.intersect(*b).is_some(),
            _ => false,
        })
    {
        let Some(merged) = rects
            .get(first)
            .zip(rects.get(second))
            .map(|(a, b)| a.hull(*b))
        else {
            break;
        };
        rects.remove(second);
        if let Some(slot) = rects.get_mut(first) {
            *slot = merged;
        }
    }
    rects
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
    /// The patches a rounded shadow draws instead of its strips, relative to its top left;
    /// none for square corners.
    patches: Vec<(Rect, Picture)>,
    /// The size, radius, corners, and offset the patches were made for.
    patched: Option<(Size, u8, u8, (i8, i8))>,
    /// The correction of the corners and radius last used, kept across resizes.
    correction: Option<((u8, u8), Vec<u8>)>,
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
        let patches: Vec<_> = self
            .shadow_patches(session, window, shadow)?
            .into_iter()
            .map(|(rect, picture)| {
                let placed = Rect {
                    left: rect.left + shadow.rect.left,
                    top: rect.top + shadow.rect.top,
                    right: rect.right + shadow.rect.left,
                    bottom: rect.bottom + shadow.rect.top,
                };
                (placed, picture)
            })
            .collect();
        let conn = &session.conn;
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
        let strips = without(
            clip.to_vec(),
            &patches.iter().map(|(rect, _)| *rect).collect::<Vec<_>>(),
        );
        if !strips.is_empty() {
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
            let strips: Vec<_> = strips
                .iter()
                .map(|rect| rect.x11())
                .collect::<Result<_>>()?;
            conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &strips)?;
            let target = area.x11()?;
            // An alpha-only source is black, so the two profiles and the strength multiply
            // into how much of it covers each pixel.
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
        }
        for (rect, picture) in patches {
            let shown = within(clip, &[rect]);
            if shown.is_empty() {
                continue;
            }
            let shown: Vec<_> = shown.iter().map(|rect| rect.x11()).collect::<Result<_>>()?;
            conn.render_set_picture_clip_rectangles(self.back.id, 0, 0, &shown)?;
            let target = rect.x11()?;
            // The black alpha mask, at the shadow's strength, through the patch's values.
            conn.render_composite(
                PictOp::OVER,
                self.alpha.id,
                picture,
                self.back.id,
                0,
                0,
                0,
                0,
                target.x,
                target.y,
                target.width,
                target.height,
            )?;
        }
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
                patches: Vec::new(),
                patched: None,
                correction: None,
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

    /// The patches `shadow`, which `window`'s surface casts, draws instead of its strips,
    /// relative to its top left. They are made again when its size, radius, corners, or
    /// offset changed, and the correction only when its corners or radius did.
    fn shadow_patches(
        &mut self,
        session: &Session,
        window: Window,
        shadow: Shadow,
    ) -> Result<Vec<(Rect, PictureId)>> {
        let kept = self
            .shadows
            .iter_mut()
            .find(|kept| kept.window == window)
            .context("a shadow has no strips")?;
        let geometry = (shadow.size, shadow.radius, shadow.corners, shadow.offset);
        if kept.patched != Some(geometry) {
            kept.patches.clear();
            if shadow.corners > 0 {
                let corners = (shadow.corners, shadow.radius);
                if kept.correction.as_ref().map(|(kept, _)| *kept) != Some(corners) {
                    kept.correction = Some((corners, correction(shadow.corners, shadow.radius)));
                }
                let correction = kept
                    .correction
                    .as_ref()
                    .map_or(&[][..], |(_, values)| values);
                for (rect, values) in patches(&shadow, correction) {
                    let (width, height) = (
                        u16::try_from(rect.right - rect.left)?,
                        u16::try_from(rect.bottom - rect.top)?,
                    );
                    let layout = (Size { width, height }, self.a8);
                    let picture = Picture::buffer(&session.conn, self.overlay, layout)?;
                    upload(session, &picture, (width, height), &values)?;
                    kept.patches.push((rect, picture));
                }
            }
            kept.patched = Some(geometry);
        }
        Ok(kept
            .patches
            .iter()
            .map(|(rect, picture)| (*rect, picture.id))
            .collect())
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

    /// How much of each pixel of `shadow`'s rounded surface lies inside it, row by row, from
    /// 0 to 255.
    fn surface(shadow: &Shadow) -> Vec<u64> {
        let (width, height) = (i32::from(shadow.size.width), i32::from(shadow.size.height));
        let corners = i32::from(shadow.corners);
        let disk = crate::renderer::corner::disk(shadow.corners);
        (0..height)
            .flat_map(|row| (0..width).map(move |column| (column, row)))
            .map(|(column, row)| {
                let (across, down) = (column.min(width - 1 - column), row.min(height - 1 - row));
                if across >= corners || down >= corners {
                    return 255;
                }
                let at = usize::try_from(down * 2 * corners + across).unwrap();
                u64::from(*disk.get(at).unwrap())
            })
            .collect()
    }

    /// The darkness at shadow pixel `(x, y)` of a direct two-dimensional blur of `shadow`'s
    /// rounded surface, whose pixels `surface` gives, as painting shows it: within the
    /// surface's corner squares, as much as the surface leaves uncovered, and `None` beneath
    /// the rest of it.
    fn blurred(shadow: &Shadow, surface: &[u64], x: i32, y: i32) -> Option<u64> {
        let (_, body) = layout(shadow);
        let (width, height) = (body.right - body.left, body.bottom - body.top);
        let inside = |column: i32, row: i32| -> u64 {
            if !(0..width).contains(&column) || !(0..height).contains(&row) {
                return 0;
            }
            *surface
                .get(usize::try_from(row * width + column).unwrap())
                .unwrap()
        };
        let (column, row) = (x - body.left, y - body.top);
        let corners = i32::from(shadow.corners);
        let beneath = (0..width).contains(&column) && (0..height).contains(&row);
        let gap = beneath
            && column.min(width - 1 - column) < corners
            && row.min(height - 1 - row) < corners;
        if beneath && !gap {
            return None;
        }
        let kernel = kernel(shadow.radius);
        let total: u64 = kernel.iter().sum();
        // Shadow pixel `x` takes weight `k` from surface pixel `x + k - 2 × radius`.
        let two = 2 * i32::from(shadow.radius);
        let tap = |index: usize| i32::try_from(index).unwrap() - two;
        let sum: u64 = kernel
            .iter()
            .enumerate()
            .flat_map(|(k, across)| {
                kernel
                    .iter()
                    .enumerate()
                    .map(move |(l, down)| across * down * inside(x + tap(k), y + tap(l)))
            })
            .sum();
        let shown = (sum + total * total / 2) / (total * total);
        Some(if gap {
            (shown * (255 - inside(column, row)) + 127) / 255
        } else {
            shown
        })
    }

    #[test]
    fn rounded_shadows_match_a_blur_of_the_rounded_surface() {
        // Given surfaces, radii, corners, and offsets, the profiles' product with the patches
        // drawn over it matches a direct two-dimensional blur of the rounded surface within
        // two levels, beneath the surface only in its corner squares, as painting shows it.
        for (width, height, radius, corners, offset) in [
            (24_u16, 18_u16, 5_u8, 6_u8, (0_i8, 0_i8)),
            (24, 18, 5, 6, (3, -2)),
            (40, 30, 8, 4, (-7, 9)),
            (12, 12, 4, 6, (0, 0)),
            (60, 20, 3, 10, (2, 2)),
            (70, 50, 6, 8, (0, 0)),
        ] {
            let shadow = Shadow {
                rect: Rect::new(0, 0, 1, 1),
                size: Size { width, height },
                radius,
                strength: u16::MAX,
                corners,
                offset,
            };
            let patched = patches(&shadow, &correction(corners, radius));
            for (index, (first, _)) in patched.iter().enumerate() {
                for (second, _) in patched.iter().skip(index + 1) {
                    assert!(first.intersect(*second).is_none(), "{shadow:?}: overlap");
                }
            }
            let (across, down) = (profile(width, radius), profile(height, radius));
            let (whole, _) = layout(&shadow);
            let pixels = surface(&shadow);
            for y in whole.top..whole.bottom {
                for x in whole.left..whole.right {
                    let patch = patched.iter().find(|(rect, _)| {
                        (rect.left..rect.right).contains(&x) && (rect.top..rect.bottom).contains(&y)
                    });
                    let expected = blurred(&shadow, &pixels, x, y);
                    let shown = match (patch, expected) {
                        (Some((rect, values)), _) => {
                            let at = (y - rect.top) * (rect.right - rect.left) + (x - rect.left);
                            u64::from(*values.get(usize::try_from(at).unwrap()).unwrap())
                        }
                        (None, None) => continue,
                        (None, Some(_)) => {
                            let at = |values: &[u8], at: i32| {
                                u64::from(*values.get(usize::try_from(at).unwrap()).unwrap())
                            };
                            (at(&across, x) * at(&down, y) + 127) / 255
                        }
                    };
                    let expected = expected.unwrap_or(0);
                    assert!(
                        shown.abs_diff(expected) <= 2,
                        "{shadow:?}: ({x}, {y}) shows {shown}, not {expected}"
                    );
                }
            }
            assert!(!patched.is_empty(), "{shadow:?}: no patches");
        }
        let square = Shadow {
            rect: Rect::new(0, 0, 1, 1),
            size: Size {
                width: 20,
                height: 20,
            },
            radius: 4,
            strength: u16::MAX,
            corners: 0,
            offset: (0, 0),
        };
        assert!(patches(&square, &correction(0, 4)).is_empty());
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
