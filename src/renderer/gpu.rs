use super::{
    blur,
    corner::{self, interior, squares, within},
    paint::{Beneath, Part, Plan},
    shadow::{self, Shadow, correction, patches},
};
use crate::{
    capabilities,
    picture::Size,
    region::{Rect, without},
    session::Session,
};
use anyhow::{Context as _, Result, bail, ensure};
use compust_gl::{
    Device, Dmabuf, Gpu, Mode, Placement, Plane, Rect as Area, Target, Texture, fourcc,
};
use std::{fs::File, os::unix::fs::MetadataExt as _, rc::Rc};
use x11rb::{
    NONE,
    connection::Connection as _,
    protocol::{
        dri3::ConnectionExt as _,
        render::Picture as PictureId,
        sync::{ConnectionExt as _, Fence},
        xproto::{Pixmap, Window},
    },
    rust_connection::RustConnection,
};

/// The background color where no wallpaper covers the root, as the `XRender` painter fills it.
const BACKGROUND: [f32; 4] = [24.0 / 255.0, 24.0 / 255.0, 32.0 / 255.0, 1.0];

/// The GPU painter. It draws each frame with OpenGL ES into the back buffer that the X server
/// allocated and Present shows, reading window pixmaps through DRI3 without copying them. The
/// kernel orders the GPU's reads and writes of shared buffers once each side has sent its
/// work: the frame is complete for the server once the draws are flushed, and before each
/// frame `fence` makes the server send the work it holds back.
pub(super) struct Painter {
    conn: Rc<RustConnection>,
    fence: Fence,
    gpu: Gpu,
    back: Target,
    /// The blur pyramid, each level half the size of the previous one.
    levels: Vec<Target>,
    wallpaper: Option<(Pixmap, Texture)>,
    /// One texture per surface capture.
    textures: Vec<(Window, PictureId, Texture)>,
    backdrops: Vec<Backdrop>,
    shadows: Vec<Shade>,
    /// One coverage disk per radius of rounded corners in use, as `corner::Masks` keeps them
    /// for the `XRender` painter.
    disks: Vec<(u8, Texture)>,
    /// One fully opaque alpha pixel, the second profile of a shadow's patches.
    one: Texture,
}

/// The textures one surface's shadow is drawn from, as `shadow::Strips` keeps them for the
/// `XRender` painter.
struct Shade {
    window: Window,
    size: Size,
    radius: u8,
    /// The profile along the x axis, one row.
    across: Texture,
    /// The profile along the y axis, one column.
    down: Texture,
    /// The patches a rounded shadow draws instead of its profiles, relative to its top left.
    patches: Vec<(Rect, Texture)>,
    /// The size, radius, corners, and offset the patches were made for.
    patched: Option<(Size, u8, u8, (i8, i8))>,
    /// The correction of the corners and radius last used, kept across resizes.
    correction: Option<((u8, u8), Vec<u8>)>,
}

/// A surface's blurred backdrop, as `blur::Backdrop` keeps it for the `XRender` painter.
struct Backdrop {
    window: Window,
    area: Rect,
    target: Target,
}

impl Painter {
    /// The GPU painter drawing into `back`, or `None`, with a warning, when the server or
    /// driver cannot provide one.
    pub(super) fn open(session: &Session, back: Pixmap, size: Size, depth: usize) -> Option<Self> {
        match session.capabilities.gpu.get() {
            capabilities::Gpu::Usable => (),
            capabilities::Gpu::Missing => {
                tracing::warn!("the X server lacks DRI3 1.2; drawing with XRender");
                return None;
            }
            capabilities::Gpu::Failed => return None,
        }
        match Self::new(session, back, size, depth) {
            Ok(painter) => Some(painter),
            Err(error) => {
                tracing::warn!("the GPU renderer is unavailable; drawing with XRender: {error:#}");
                None
            }
        }
    }

    fn new(session: &Session, back: Pixmap, size: Size, depth: usize) -> Result<Self> {
        let conn = &session.conn;
        let device = conn.dri3_open(session.screen.root, NONE)?.reply()?;
        let node = File::from(device.device_fd).metadata()?;
        let gpu = Gpu::open(Device::Drm(node.rdev()))?;
        let back = gpu.import_target(&dmabuf(conn, back)?)?;
        let fence = conn.generate_id()?;
        conn.sync_create_fence(session.screen.root, fence, false)?
            .check()?;
        let mut levels = Vec::with_capacity(depth);
        let (mut width, mut height) = (u32::from(size.width), u32::from(size.height));
        for _ in 0..depth {
            (width, height) = (width.div_ceil(2), height.div_ceil(2));
            levels.push(gpu.target(width, height)?);
        }
        tracing::info!(renderer = gpu.renderer()?, "drawing with the GPU");
        let one = gpu.alpha(1, 1, &[u8::MAX])?;
        Ok(Self {
            conn: Rc::clone(conn),
            fence,
            gpu,
            back,
            levels,
            wallpaper: None,
            textures: Vec::new(),
            backdrops: Vec::new(),
            shadows: Vec::new(),
            disks: Vec::new(),
            one,
        })
    }

    /// Whether the backdrop kept for `window` covers `bounds`.
    pub(super) fn kept(&self, window: Window, bounds: Rect) -> bool {
        self.backdrops
            .iter()
            .any(|kept| kept.window == window && kept.area == bounds)
    }

    /// Drop the backdrops of windows not in `windows`.
    pub(super) fn retain(&mut self, windows: &[Window]) {
        self.backdrops.retain(|kept| windows.contains(&kept.window));
    }

    pub(super) fn forget(&mut self, window: Window) {
        self.backdrops.retain(|kept| kept.window != window);
    }

    pub(super) fn forget_wallpaper(&mut self) {
        self.wallpaper = None;
    }

    /// Draw `plan` into the back buffer and flush it, with `wallpaper` as the root's pixmap.
    pub(super) fn paint(&mut self, plan: &Plan<'_>, wallpaper: Option<Pixmap>) -> Result<()> {
        self.import(plan, wallpaper)?;
        // The server sends its own GPU work, such as a window's newly painted background or
        // the copy that sharing a pixmap can need, when it idles or a fence triggers. Once the
        // trigger is answered, the kernel orders this frame's reads after that work.
        self.conn.sync_trigger_fence(self.fence)?;
        self.conn.sync_query_fence(self.fence)?.reply()?;
        self.conn.sync_reset_fence(self.fence)?;
        let background = areas(&plan.background)?;
        if !background.is_empty() {
            let mut frame = self.gpu.frame(&self.back)?;
            frame.fill(BACKGROUND, &background)?;
            if let Some((_, texture)) = &self.wallpaper {
                frame.draw(texture, Placement::At(0, 0), Mode::Replace, &background)?;
            }
        }
        self.shadows.retain(|kept| {
            plan.parts
                .iter()
                .any(|part| part.shadow.is_some() && part.surface.window == kept.window)
        });
        self.disks
            .retain(|(radius, _)| plan.parts.iter().any(|part| part.corners == *radius));
        for part in plan.parts.iter().filter(|part| part.corners > 0) {
            if !self.disks.iter().any(|(radius, _)| *radius == part.corners) {
                let side = 2 * u32::from(part.corners);
                let disk = self.gpu.alpha(side, side, &corner::disk(part.corners))?;
                self.disks.push((part.corners, disk));
            }
        }
        for part in &plan.parts {
            let window = part.surface.window;
            // A backdrop due to blur again is refreshed even where none of it shows, so that a
            // later frame never reuses a stale one.
            if let Beneath::Fresh { bounds, footprint } = part.beneath {
                self.keep(window, bounds)?;
                self.blur(window, footprint)?;
            }
            // The shadow follows the blur, which reads the scene beneath the surface without it.
            if let Some((shadow, around)) = &part.shadow
                && !around.is_empty()
            {
                self.shade(window, *shadow, around)?;
            }
            if !part.clip.is_empty() {
                self.draw_surface(part)?;
            }
        }
        self.gpu.flush()
    }

    /// Import the pixmaps `plan` shows, and drop textures it no longer needs.
    fn import(&mut self, plan: &Plan<'_>, wallpaper: Option<Pixmap>) -> Result<()> {
        let conn = &*self.conn;
        self.textures.retain(|(window, picture, _)| {
            plan.parts
                .iter()
                .any(|part| part.surface.window == *window && part.surface.picture.id == *picture)
        });
        for part in plan.parts.iter().filter(|part| !part.clip.is_empty()) {
            let (window, picture) = (part.surface.window, part.surface.picture.id);
            if !self.textures.iter().any(|(kept, _, _)| *kept == window) {
                let pixmap = part
                    .surface
                    .picture
                    .pixmap
                    .context("a surface has no pixmap")?;
                let texture = self.gpu.import(&dmabuf(conn, pixmap)?)?;
                self.textures.push((window, picture, texture));
            }
        }
        if self.wallpaper.as_ref().map(|(pixmap, _)| *pixmap) != wallpaper {
            self.wallpaper = None;
            if let Some(pixmap) = wallpaper {
                let texture = self.gpu.import(&dmabuf(conn, pixmap)?)?;
                texture.repeat()?;
                self.wallpaper = Some((pixmap, texture));
            }
        }
        Ok(())
    }

    /// Draw `shadow`, which `window`'s surface casts, within `around`, from the profiles the
    /// `XRender` painter uses.
    fn shade(&mut self, window: Window, shadow: Shadow, around: &[Rect]) -> Result<()> {
        let kept = |kept: &&Shade| {
            kept.window == window && kept.size == shadow.size && kept.radius == shadow.radius
        };
        if !self.shadows.iter().any(|shade| kept(&shade)) {
            let across = shadow::profile(shadow.size.width, shadow.radius);
            let down = shadow::profile(shadow.size.height, shadow.radius);
            let shade = Shade {
                window,
                size: shadow.size,
                radius: shadow.radius,
                across: self.gpu.alpha(u32::try_from(across.len())?, 1, &across)?,
                down: self.gpu.alpha(1, u32::try_from(down.len())?, &down)?,
                patches: Vec::new(),
                patched: None,
                correction: None,
            };
            self.shadows.retain(|kept| kept.window != window);
            self.shadows.push(shade);
        }
        let shade = self
            .shadows
            .iter_mut()
            .find(|shade| kept(&&**shade))
            .context("a shadow has no textures")?;
        let geometry = (shadow.size, shadow.radius, shadow.corners, shadow.offset);
        if shade.patched != Some(geometry) {
            shade.patches.clear();
            if shadow.corners > 0 {
                let corners = (shadow.corners, shadow.radius);
                if shade.correction.as_ref().map(|(kept, _)| *kept) != Some(corners) {
                    shade.correction = Some((corners, correction(shadow.corners, shadow.radius)));
                }
                let values = shade
                    .correction
                    .as_ref()
                    .map_or(&[][..], |(_, values)| values);
                for (rect, values) in patches(&shadow, values) {
                    let (width, height) = (
                        u32::try_from(rect.right - rect.left)?,
                        u32::try_from(rect.bottom - rect.top)?,
                    );
                    shade
                        .patches
                        .push((rect, self.gpu.alpha(width, height, &values)?));
                }
            }
            shade.patched = Some(geometry);
        }
        let strength = f32::from(shadow.strength) / f32::from(u16::MAX);
        let placed: Vec<_> = shade
            .patches
            .iter()
            .map(|(rect, texture)| {
                let rect = Rect {
                    left: rect.left + shadow.rect.left,
                    top: rect.top + shadow.rect.top,
                    right: rect.right + shadow.rect.left,
                    bottom: rect.bottom + shadow.rect.top,
                };
                (rect, texture)
            })
            .collect();
        let mut frame = self.gpu.frame(&self.back)?;
        let strips = without(
            around.to_vec(),
            &placed.iter().map(|(rect, _)| *rect).collect::<Vec<_>>(),
        );
        if !strips.is_empty() {
            frame.shade(
                (&shade.across, &shade.down),
                (shadow.rect.left, shadow.rect.top),
                strength,
                &areas(&strips)?,
            )?;
        }
        // A patch holds both axes; the opaque pixel stands for the second profile.
        for (rect, texture) in placed {
            let shown = within(around, &[rect]);
            if !shown.is_empty() {
                frame.shade(
                    (texture, &self.one),
                    (rect.left, rect.top),
                    strength,
                    &areas(&shown)?,
                )?;
            }
        }
        Ok(())
    }

    /// Draw `part`'s surface within its clip, over its blurred backdrop where it has one.
    /// With rounded corners, its interior is drawn as a square surface is, and each corner
    /// square through its quadrant of the disk, as the `XRender` painter draws them.
    fn draw_surface(&self, part: &Part<'_>) -> Result<()> {
        let window = part.surface.window;
        let texture = self
            .textures
            .iter()
            .find(|(kept, _, _)| *kept == window)
            .map(|(_, _, texture)| texture)
            .context("a surface was not imported")?;
        let origin = (
            i32::from(part.surface.geometry.x),
            i32::from(part.surface.geometry.y),
        );
        let opacity = f32::from(part.opacity) / f32::from(u16::MAX);
        let backdrop = match part.beneath {
            Beneath::Kept(bounds) | Beneath::Fresh { bounds, .. } => self
                .backdrops
                .iter()
                .find(|kept| kept.window == window && kept.area == bounds),
            Beneath::Scene => None,
        };
        let bounds = part.surface.bounds();
        let inner = within(&part.clip, &interior(bounds, part.corners));
        let mut frame = self.gpu.frame(&self.back)?;
        if !inner.is_empty() {
            let inner = areas(&inner)?;
            if let Some(backdrop) = backdrop {
                // The blur shows as strongly as the surface covers each pixel, as the XRender
                // painter weighs it.
                let at = Placement::At(backdrop.area.left, backdrop.area.top);
                frame.draw_masked(
                    backdrop.target.texture(),
                    at,
                    (texture, origin),
                    opacity,
                    &inner,
                )?;
            }
            let at = Placement::At(origin.0, origin.1);
            frame.draw(texture, at, Mode::Over(opacity), &inner)?;
        }
        if part.corners == 0 {
            return Ok(());
        }
        let disk = self
            .disks
            .iter()
            .find(|(radius, _)| *radius == part.corners)
            .map(|(_, disk)| disk)
            .context("rounded corners have no disk")?;
        for (square, (across, down)) in squares(bounds, part.corners) {
            let shown = within(&part.clip, &[square]);
            if shown.is_empty() {
                continue;
            }
            let shown = areas(&shown)?;
            let disk_at = (square.left - across, square.top - down);
            if let Some(backdrop) = backdrop {
                let at = Placement::At(backdrop.area.left, backdrop.area.top);
                frame.draw_masked_covered(
                    backdrop.target.texture(),
                    at,
                    (texture, origin),
                    (disk, disk_at),
                    opacity,
                    &shown,
                )?;
            }
            let at = Placement::At(origin.0, origin.1);
            frame.draw_covered(texture, at, (disk, disk_at), opacity, &shown)?;
        }
        Ok(())
    }

    /// Make the backdrop of `window` hold `area`, as `Renderer::keep` does for `XRender`.
    fn keep(&mut self, window: Window, area: Rect) -> Result<()> {
        let (width, height) = (
            u32::try_from(area.right - area.left)?,
            u32::try_from(area.bottom - area.top)?,
        );
        match self.backdrops.iter_mut().find(|kept| kept.window == window) {
            Some(kept) if fits(kept.target.texture(), width, height) => kept.area = area,
            _ => {
                let round = |value: u32| value.div_ceil(64).saturating_mul(64);
                let target = self.gpu.target(round(width), round(height))?;
                self.backdrops.retain(|kept| kept.window != window);
                self.backdrops.push(Backdrop {
                    window,
                    area,
                    target,
                });
            }
        }
        Ok(())
    }

    /// Blur the scene in the back buffer across `footprint` into the backdrop of `window`, with
    /// the passes of the `XRender` painter's pyramid.
    fn blur(&self, window: Window, footprint: Rect) -> Result<()> {
        let backdrop = self
            .backdrops
            .iter()
            .find(|kept| kept.window == window)
            .context("a blurred surface has no backdrop")?;
        let halve = Placement::Scaled {
            offset: [0, 0],
            scale: 2.0,
        };
        let double = Placement::Scaled {
            offset: [0, 0],
            scale: 0.5,
        };
        let mut source = self.back.texture();
        for (index, level) in self.levels.iter().enumerate() {
            let area = level_area(footprint, index + 1)?;
            self.gpu
                .frame(level)?
                .draw(source, halve, Mode::Replace, &[area])?;
            source = level.texture();
        }
        for (index, pair) in self.levels.windows(2).enumerate().rev() {
            if let [finer, coarser] = pair {
                let area = level_area(footprint, index + 1)?;
                self.gpu
                    .frame(finer)?
                    .draw(coarser.texture(), double, Mode::Replace, &[area])?;
            }
        }
        if let Some(finest) = self.levels.first() {
            let area = backdrop.area;
            let whole = Area {
                x: 0,
                y: 0,
                width: area.right - area.left,
                height: area.bottom - area.top,
            };
            let scaled = Placement::Scaled {
                offset: [area.left, area.top],
                scale: 0.5,
            };
            self.gpu.frame(&backdrop.target)?.draw(
                finest.texture(),
                scaled,
                Mode::Replace,
                &[whole],
            )?;
        }
        Ok(())
    }
}

/// Whether a backdrop texture holds `width` by `height` without wasting over four times that.
fn fits(texture: &Texture, width: u32, height: u32) -> bool {
    let area = |width: u32, height: u32| u64::from(width) * u64::from(height);
    texture.width() >= width
        && texture.height() >= height
        && area(texture.width(), texture.height()) <= 4 * area(width, height)
}

/// The buffer of `pixmap`, shared through DRI3.
fn dmabuf(conn: &RustConnection, pixmap: Pixmap) -> Result<Dmabuf> {
    let reply = conn.dri3_buffers_from_pixmap(pixmap)?.reply()?;
    let format = match (reply.depth, reply.bpp) {
        (24, 32) => fourcc::XRGB8888,
        (32, 32) => fourcc::ARGB8888,
        (depth, bpp) => bail!("pixmaps of depth {depth} at {bpp} bits per pixel cannot be shared"),
    };
    ensure!(
        reply.buffers.len() == reply.strides.len() && reply.buffers.len() == reply.offsets.len(),
        "DRI3 described a buffer inconsistently"
    );
    let planes = reply
        .buffers
        .into_iter()
        .zip(reply.strides)
        .zip(reply.offsets)
        .map(|((fd, stride), offset)| Plane { fd, offset, stride })
        .collect();
    Ok(Dmabuf {
        width: u32::from(reply.width),
        height: u32::from(reply.height),
        format,
        modifier: reply.modifier,
        planes,
    })
}

fn areas(rects: &[Rect]) -> Result<Vec<Area>> {
    rects
        .iter()
        .map(|rect| {
            Ok(Area {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            })
        })
        .collect()
}

/// `footprint` in the coordinates of pyramid `level`.
fn level_area(footprint: Rect, level: usize) -> Result<Area> {
    let area = blur::level_area(footprint, level)?;
    Ok(Area {
        x: i32::from(area.x),
        y: i32::from(area.y),
        width: i32::from(area.width),
        height: i32::from(area.height),
    })
}

impl Drop for Painter {
    fn drop(&mut self) {
        if let Err(error) = self
            .conn
            .sync_destroy_fence(self.fence)
            .map(x11rb::cookie::VoidCookie::ignore_error)
        {
            tracing::debug!(%error, "fence cleanup failed");
        }
    }
}
