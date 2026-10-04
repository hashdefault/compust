use crate::device::{Gpu, Inner};
use anyhow::{Context as _, Result, ensure};
use glow::HasContext as _;
use khronos_egl as egl;
use std::{
    os::fd::{AsRawFd as _, OwnedFd},
    ptr::null_mut,
    rc::Rc,
};

/// DRM format codes for the buffers X pixmaps use.
pub mod fourcc {
    /// 32 bits per pixel with an unused top byte: depth-24 pixmaps.
    pub const XRGB8888: u32 = u32::from_le_bytes(*b"XR24");
    /// 32 bits per pixel with premultiplied alpha in the top byte: depth-32 pixmaps.
    pub const ARGB8888: u32 = u32::from_le_bytes(*b"AR24");
}

/// The modifier of a buffer whose layout no modifier describes, as DRI3 reports it.
pub const MOD_INVALID: u64 = 0x00ff_ffff_ffff_ffff;

const LINUX_DMA_BUF: egl::Enum = 0x3270;
const LINUX_DRM_FOURCC: egl::Attrib = 0x3271;
/// File descriptor, offset, and pitch attributes of planes 0 to 3.
const PLANES: [[egl::Attrib; 3]; 4] = [
    [0x3272, 0x3273, 0x3274],
    [0x3275, 0x3276, 0x3277],
    [0x3278, 0x3279, 0x327A],
    [0x3440, 0x3441, 0x3442],
];
/// Low and high modifier halves of planes 0 to 3.
const MODIFIERS: [[egl::Attrib; 2]; 4] = [
    [0x3443, 0x3444],
    [0x3445, 0x3446],
    [0x3447, 0x3448],
    [0x3449, 0x344A],
];

/// One plane of a dma-buf.
#[derive(Debug)]
pub struct Plane {
    pub fd: OwnedFd,
    pub offset: u32,
    pub stride: u32,
}

/// A buffer shared through dma-buf file descriptors, as DRI3 describes a pixmap.
#[derive(Debug)]
pub struct Dmabuf {
    pub width: u32,
    pub height: u32,
    /// A `fourcc` code.
    pub format: u32,
    /// The layout modifier, or `MOD_INVALID` for one the driver infers.
    pub modifier: u64,
    pub planes: Vec<Plane>,
}

/// A texture the GPU samples: imported from a dma-buf, or the color buffer of a `Target`.
/// Texture coordinates run from the first row in memory, which X calls the top.
pub struct Texture {
    pub(crate) inner: Rc<Inner>,
    pub(crate) id: glow::Texture,
    image: Option<egl::Image>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Texture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Tile the texture beyond its edges instead of clamping, as a wallpaper does.
    pub fn repeat(&self) -> Result<()> {
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context that owns the texture current; the calls name the
        // texture and constants.
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
            for wrap in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, wrap, glow::REPEAT.cast_signed());
            }
        }
        Ok(())
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        // Without the context, the texture goes when it is destroyed.
        if let Ok(gl) = self.inner.gl() {
            // SAFETY: `gl` made the context that owns the texture current.
            unsafe { gl.delete_texture(self.id) };
        }
        if let Some(image) = self.image {
            let _destroyed = self.inner.egl.destroy_image(self.inner.display, image);
        }
    }
}

/// A texture that can be drawn into.
pub struct Target {
    pub(crate) framebuffer: glow::Framebuffer,
    pub(crate) texture: Texture,
}

impl Target {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        if let Ok(gl) = self.texture.inner.gl() {
            // SAFETY: `gl` made the context that owns the framebuffer current.
            unsafe { gl.delete_framebuffer(self.framebuffer) };
        }
    }
}

impl Gpu {
    /// Sample `buffer` without copying it. The texture shows whatever the buffer holds when
    /// a draw reads it; the kernel orders those reads after earlier writes to the buffer.
    pub fn import(&self, buffer: &Dmabuf) -> Result<Texture> {
        let inner = &self.inner;
        ensure!(
            !buffer.planes.is_empty() && buffer.planes.len() <= PLANES.len(),
            "a dma-buf needs one to four planes"
        );
        ensure!(
            buffer.modifier == MOD_INVALID || inner.modifiers,
            "EGL cannot import format modifiers"
        );
        let mut attributes = vec![
            usize::try_from(egl::WIDTH)?,
            usize::try_from(buffer.width)?,
            usize::try_from(egl::HEIGHT)?,
            usize::try_from(buffer.height)?,
            LINUX_DRM_FOURCC,
            usize::try_from(buffer.format)?,
        ];
        for ((plane, names), modifier) in buffer.planes.iter().zip(PLANES).zip(MODIFIERS) {
            let [fd, offset, pitch] = names;
            attributes.extend([
                fd,
                usize::try_from(plane.fd.as_raw_fd())?,
                offset,
                usize::try_from(plane.offset)?,
                pitch,
                usize::try_from(plane.stride)?,
            ]);
            if buffer.modifier != MOD_INVALID {
                let [low, high] = modifier;
                attributes.extend([
                    low,
                    usize::try_from(buffer.modifier & 0xffff_ffff)?,
                    high,
                    usize::try_from(buffer.modifier >> 32)?,
                ]);
            }
        }
        attributes.push(egl::ATTRIB_NONE);
        // SAFETY: EGL_LINUX_DMA_BUF_EXT images take no context and no client buffer, both of
        // which are null.
        let (context, client) = unsafe {
            (
                egl::Context::from_ptr(egl::NO_CONTEXT),
                egl::ClientBuffer::from_ptr(null_mut()),
            )
        };
        // EGL duplicates the descriptors, which `buffer` keeps open during the call.
        let image = inner
            .egl
            .create_image(inner.display, context, LINUX_DMA_BUF, client, &attributes)
            .context("importing a dma-buf into EGL")?;
        let texture = Texture {
            inner: Rc::clone(inner),
            id: self.new_texture()?,
            image: Some(image),
            width: buffer.width,
            height: buffer.height,
        };
        let gl = inner.gl()?;
        // SAFETY: `gl` made the context current, `new_texture` bound the texture to TEXTURE_2D,
        // and the image is a live EGL image of this display.
        unsafe { (inner.image_target_texture)(glow::TEXTURE_2D, image.as_ptr()) };
        // SAFETY: as above; GetError takes no arguments.
        let error = unsafe { gl.get_error() };
        ensure!(
            error == glow::NO_ERROR,
            "binding a dma-buf to a texture failed: {error:#x}"
        );
        Ok(texture)
    }

    /// Import `buffer` as a texture to draw into.
    pub fn import_target(&self, buffer: &Dmabuf) -> Result<Target> {
        let texture = self.import(buffer)?;
        self.framebuffer(texture)
    }

    /// A new texture of `width` by `height` premultiplied RGBA pixels to draw into.
    pub fn target(&self, width: u32, height: u32) -> Result<Target> {
        let texture = Texture {
            inner: Rc::clone(&self.inner),
            id: self.new_texture()?,
            image: None,
            width,
            height,
        };
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current and `new_texture` bound the texture; null data
        // leaves its contents undefined, and the sizes are within GL's signed range.
        unsafe {
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGBA8)?,
                i32::try_from(width)?,
                i32::try_from(height)?,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
        }
        self.framebuffer(texture)
    }

    /// A texture name bound to `TEXTURE_2D`, clamped at its edges.
    fn new_texture(&self) -> Result<glow::Texture> {
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current; the calls name only the new texture and
        // constants.
        unsafe {
            let texture = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for wrap in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, wrap, glow::CLAMP_TO_EDGE.cast_signed());
            }
            Ok(texture)
        }
    }

    fn framebuffer(&self, texture: Texture) -> Result<Target> {
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current; the calls name only the new framebuffer, the
        // live texture, and constants.
        let (framebuffer, status) = unsafe {
            let framebuffer = gl.create_framebuffer().map_err(anyhow::Error::msg)?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.id),
                0,
            );
            (framebuffer, gl.check_framebuffer_status(glow::FRAMEBUFFER))
        };
        let target = Target {
            framebuffer,
            texture,
        };
        ensure!(
            status == glow::FRAMEBUFFER_COMPLETE,
            "the texture cannot be drawn into: {status:#x}"
        );
        Ok(target)
    }
}
