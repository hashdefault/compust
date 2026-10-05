use crate::{
    device::{Gpu, Inner},
    texture::{Target, Texture},
};
use anyhow::{Context as _, Result, ensure};
use glow::HasContext as _;
use std::rc::Rc;

type Location = <glow::Context as glow::HasContext>::UniformLocation;

/// A rectangle of pixels, from the top left in X's orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// How a draw combines with the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Replace the target's pixels.
    Replace,
    /// Composite premultiplied pixels over the target at an opacity from 0 to 1.
    Over(f32),
}

/// Where a draw reads its source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    /// Source pixel (0, 0) lands on target pixel (x, y), one to one.
    At(i32, i32),
    /// Target pixel `p` samples the source at pixel `(p + offset) * scale`, between texels
    /// with bilinear filtering, as an `XRender` transform scales a picture.
    Scaled { offset: [i32; 2], scale: f32 },
}

const VERTEX: &str = "
attribute vec2 position;
uniform vec2 viewport;
uniform vec4 mapping;
uniform vec4 mask_mapping;
uniform vec4 cover_mapping;
varying vec2 texcoord;
varying vec2 maskcoord;
varying vec2 covercoord;
void main() {
    texcoord = position * mapping.xy + mapping.zw;
    maskcoord = position * mask_mapping.xy + mask_mapping.zw;
    covercoord = position * cover_mapping.xy + cover_mapping.zw;
    gl_Position = vec4(position / viewport * 2.0 - 1.0, 0.0, 1.0);
}
";

const PRECISION: &str = "
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
";

const TEXTURED: &str = "
uniform sampler2D source;
uniform float opacity;
varying vec2 texcoord;
void main() {
    gl_FragColor = texture2D(source, texcoord) * opacity;
}
";

const SOLID: &str = "
uniform vec4 color;
varying vec2 texcoord;
void main() {
    gl_FragColor = color;
}
";

const MASKED: &str = "
uniform sampler2D source;
uniform sampler2D mask;
uniform float opacity;
varying vec2 texcoord;
varying vec2 maskcoord;
void main() {
    float weight = texture2D(mask, maskcoord).a * opacity;
    gl_FragColor = vec4(texture2D(source, texcoord).rgb, 1.0) * weight;
}
";

const COVERED: &str = "
uniform sampler2D source;
uniform sampler2D mask;
uniform float opacity;
varying vec2 texcoord;
varying vec2 maskcoord;
void main() {
    gl_FragColor = texture2D(source, texcoord) * (texture2D(mask, maskcoord).a * opacity);
}
";

const MASKED_COVERED: &str = "
uniform sampler2D source;
uniform sampler2D mask;
uniform sampler2D cover;
uniform float opacity;
varying vec2 texcoord;
varying vec2 maskcoord;
varying vec2 covercoord;
void main() {
    float weight = texture2D(mask, maskcoord).a * texture2D(cover, covercoord).a * opacity;
    gl_FragColor = vec4(texture2D(source, texcoord).rgb, 1.0) * weight;
}
";

const SHADED: &str = "
uniform sampler2D source;
uniform sampler2D mask;
uniform float opacity;
varying vec2 texcoord;
varying vec2 maskcoord;
void main() {
    float weight = texture2D(source, texcoord).a * texture2D(mask, maskcoord).a * opacity;
    gl_FragColor = vec4(0.0, 0.0, 0.0, weight);
}
";

struct Program {
    id: glow::Program,
    viewport: Option<Location>,
    mapping: Option<Location>,
    mask_mapping: Option<Location>,
    cover_mapping: Option<Location>,
    opacity: Option<Location>,
    color: Option<Location>,
}

/// The shaders and vertex buffer every draw uses.
pub(crate) struct Programs {
    inner: Rc<Inner>,
    textured: Program,
    solid: Program,
    masked: Program,
    covered: Program,
    masked_covered: Program,
    shaded: Program,
    vertices: glow::Buffer,
}

impl Programs {
    pub(crate) fn new(inner: &Rc<Inner>) -> Result<Self> {
        let textured = program(inner, TEXTURED)?;
        let solid = program(inner, SOLID)?;
        let masked = program(inner, MASKED)?;
        let covered = program(inner, COVERED)?;
        let masked_covered = program(inner, MASKED_COVERED)?;
        let shaded = program(inner, SHADED)?;
        let gl = inner.gl()?;
        // SAFETY: `gl` made the context current.
        let vertices = unsafe { gl.create_buffer() }.map_err(anyhow::Error::msg)?;
        Ok(Self {
            inner: Rc::clone(inner),
            textured,
            solid,
            masked,
            covered,
            masked_covered,
            shaded,
            vertices,
        })
    }
}

impl Drop for Programs {
    fn drop(&mut self) {
        // Without the context, the objects go when it is destroyed.
        if let Ok(gl) = self.inner.gl() {
            // SAFETY: `gl` made the context that owns these objects current.
            unsafe {
                gl.delete_program(self.textured.id);
                gl.delete_program(self.solid.id);
                gl.delete_program(self.masked.id);
                gl.delete_program(self.covered.id);
                gl.delete_program(self.masked_covered.id);
                gl.delete_program(self.shaded.id);
                gl.delete_buffer(self.vertices);
            }
        }
    }
}

fn program(inner: &Inner, fragment: &str) -> Result<Program> {
    let gl = inner.gl()?;
    // SAFETY: `gl` made the context current; the calls name only the objects created here.
    unsafe {
        let id = gl.create_program().map_err(anyhow::Error::msg)?;
        for (kind, source) in [
            (glow::VERTEX_SHADER, VERTEX.to_owned()),
            (glow::FRAGMENT_SHADER, format!("{PRECISION}{fragment}")),
        ] {
            let shader = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
            gl.shader_source(shader, &source);
            gl.compile_shader(shader);
            let compiled = gl.get_shader_compile_status(shader);
            let log = gl.get_shader_info_log(shader);
            gl.attach_shader(id, shader);
            gl.delete_shader(shader);
            ensure!(compiled, "compiling a shader: {log}");
        }
        gl.bind_attrib_location(id, 0, "position");
        gl.link_program(id);
        ensure!(
            gl.get_program_link_status(id),
            "linking a shader program: {}",
            gl.get_program_info_log(id)
        );
        // Sources sample texture unit 0, masks unit 1, and coverage unit 2.
        gl.use_program(Some(id));
        gl.uniform_1_i32(gl.get_uniform_location(id, "source").as_ref(), 0);
        gl.uniform_1_i32(gl.get_uniform_location(id, "mask").as_ref(), 1);
        gl.uniform_1_i32(gl.get_uniform_location(id, "cover").as_ref(), 2);
        Ok(Program {
            id,
            viewport: gl.get_uniform_location(id, "viewport"),
            mapping: gl.get_uniform_location(id, "mapping"),
            mask_mapping: gl.get_uniform_location(id, "mask_mapping"),
            cover_mapping: gl.get_uniform_location(id, "cover_mapping"),
            opacity: gl.get_uniform_location(id, "opacity"),
            color: gl.get_uniform_location(id, "color"),
        })
    }
}

/// Drawing into one target. Draws reach the GPU in order; `Gpu::flush` sends them.
pub struct Frame<'a> {
    gpu: &'a Gpu,
    target: &'a Target,
}

impl Gpu {
    pub fn frame<'a>(&'a self, target: &'a Target) -> Result<Frame<'a>> {
        let (width, height) = (
            i32::try_from(target.texture.width)?,
            i32::try_from(target.texture.height)?,
        );
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current, and the framebuffer belongs to it.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            gl.viewport(0, 0, width, height);
        }
        Ok(Frame { gpu: self, target })
    }

    /// Send the draws so far to the GPU. Another client using a buffer they wrote, such as
    /// the X server presenting it, then sees them. A GL error since the last flush, such as a
    /// lost device, fails it, so the caller can stop using the GPU.
    pub fn flush(&self) -> Result<()> {
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current, and Flush and GetError take no arguments.
        let error = unsafe {
            gl.flush();
            gl.get_error()
        };
        ensure!(error == glow::NO_ERROR, "the GPU reported error {error:#x}");
        Ok(())
    }

    /// Copy `area` of `target` to the CPU, as RGBA rows from the top. This waits for the GPU,
    /// so it serves tests and diagnostics, not frames.
    pub fn read(&self, target: &Target, area: Rect) -> Result<Vec<u8>> {
        let length = usize::try_from(area.width)?
            .checked_mul(usize::try_from(area.height)?)
            .and_then(|pixels| pixels.checked_mul(4))
            .context("read area too large")?;
        let mut pixels = vec![0; length];
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current, and `pixels` holds the 4 bytes per pixel of `area`
        // that RGBA with unsigned bytes writes, with the default pack alignment of 4.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            gl.read_pixels(
                area.x,
                area.y,
                area.width,
                area.height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        Ok(pixels)
    }
}

impl Frame<'_> {
    /// Replace `clip` with `color`, premultiplied RGBA from 0 to 1.
    pub fn fill(&mut self, color: [f32; 4], clip: &[Rect]) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.solid;
        let gl = self.gpu.inner.gl()?;
        let [red, green, blue, alpha] = color;
        // SAFETY: `gl` made the context current; the program and its uniforms belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            gl.uniform_4_f32(program.color.as_ref(), red, green, blue, alpha);
            gl.disable(glow::BLEND);
        }
        self.quads(program, clip)
    }

    /// Draw `source` into `clip` as `placement` maps it, combined as `mode` says.
    pub fn draw(
        &mut self,
        source: &Texture,
        placement: Placement,
        mode: Mode,
        clip: &[Rect],
    ) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.textured;
        let gl = self.gpu.inner.gl()?;
        let (mapping, filter) = mapping(source, placement)?;
        let (opacity, blend) = match mode {
            Mode::Replace => (1.0, false),
            Mode::Over(opacity) => (opacity, true),
        };
        // SAFETY: `gl` made the context current; the program, its uniforms, and the texture
        // belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(source.id));
            for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, filter.cast_signed());
            }
            gl.uniform_4_f32_slice(program.mapping.as_ref(), &mapping);
            gl.uniform_1_f32(program.opacity.as_ref(), opacity);
            if blend {
                gl.enable(glow::BLEND);
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            } else {
                gl.disable(glow::BLEND);
            }
        }
        self.quads(program, clip)
    }

    /// Composite `source`, mapped by `placement` and taken as opaque, over `clip`, weighted
    /// at each pixel by the alpha of `mask` times `opacity`. The mask's pixel (0, 0) lies on
    /// target pixel `mask_at`, one to one.
    pub fn draw_masked(
        &mut self,
        source: &Texture,
        placement: Placement,
        (mask, mask_at): (&Texture, (i32, i32)),
        opacity: f32,
        clip: &[Rect],
    ) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.masked;
        let gl = self.gpu.inner.gl()?;
        let (mask_mapping, _) = mapping(mask, Placement::At(mask_at.0, mask_at.1))?;
        let (mapping, filter) = mapping(source, placement)?;
        // SAFETY: `gl` made the context current; the program, its uniforms, and both textures
        // belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            for (unit, texture, filter) in [
                (glow::TEXTURE1, mask, glow::NEAREST),
                (glow::TEXTURE0, source, filter),
            ] {
                gl.active_texture(unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
                for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, filter.cast_signed());
                }
            }
            gl.uniform_4_f32_slice(program.mapping.as_ref(), &mapping);
            gl.uniform_4_f32_slice(program.mask_mapping.as_ref(), &mask_mapping);
            gl.uniform_1_f32(program.opacity.as_ref(), opacity);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.quads(program, clip)
    }

    /// Composite premultiplied `source`, mapped by `placement`, over `clip`, weighted at each
    /// pixel by the alpha of `mask` times `opacity`, as the `XRender` painter draws the corners
    /// of a rounded window through its disk. The mask's pixel (0, 0) lies on target pixel
    /// `mask_at`, one to one.
    pub fn draw_covered(
        &mut self,
        source: &Texture,
        placement: Placement,
        (mask, mask_at): (&Texture, (i32, i32)),
        opacity: f32,
        clip: &[Rect],
    ) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.covered;
        let gl = self.gpu.inner.gl()?;
        let (mask_mapping, _) = mapping(mask, Placement::At(mask_at.0, mask_at.1))?;
        let (mapping, filter) = mapping(source, placement)?;
        // SAFETY: `gl` made the context current; the program, its uniforms, and both textures
        // belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            for (unit, texture, filter) in [
                (glow::TEXTURE1, mask, glow::NEAREST),
                (glow::TEXTURE0, source, filter),
            ] {
                gl.active_texture(unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
                for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, filter.cast_signed());
                }
            }
            gl.uniform_4_f32_slice(program.mapping.as_ref(), &mapping);
            gl.uniform_4_f32_slice(program.mask_mapping.as_ref(), &mask_mapping);
            gl.uniform_1_f32(program.opacity.as_ref(), opacity);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.quads(program, clip)
    }

    /// `draw_masked`, with the weight at each pixel also multiplied by the alpha of `cover`,
    /// whose pixel (0, 0) lies on target pixel `cover_at`: the blurred backdrop beneath the
    /// corners of a rounded window.
    pub fn draw_masked_covered(
        &mut self,
        source: &Texture,
        placement: Placement,
        (mask, mask_at): (&Texture, (i32, i32)),
        (cover, cover_at): (&Texture, (i32, i32)),
        opacity: f32,
        clip: &[Rect],
    ) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.masked_covered;
        let gl = self.gpu.inner.gl()?;
        let (cover_mapping, _) = mapping(cover, Placement::At(cover_at.0, cover_at.1))?;
        let (mask_mapping, _) = mapping(mask, Placement::At(mask_at.0, mask_at.1))?;
        let (mapping, filter) = mapping(source, placement)?;
        // SAFETY: `gl` made the context current; the program, its uniforms, and the three
        // textures belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            for (unit, texture, filter) in [
                (glow::TEXTURE2, cover, glow::NEAREST),
                (glow::TEXTURE1, mask, glow::NEAREST),
                (glow::TEXTURE0, source, filter),
            ] {
                gl.active_texture(unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
                for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, filter.cast_signed());
                }
            }
            gl.uniform_4_f32_slice(program.mapping.as_ref(), &mapping);
            gl.uniform_4_f32_slice(program.mask_mapping.as_ref(), &mask_mapping);
            gl.uniform_4_f32_slice(program.cover_mapping.as_ref(), &cover_mapping);
            gl.uniform_1_f32(program.opacity.as_ref(), opacity);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.quads(program, clip)
    }

    /// Darken `clip` toward black as a shadow does: at each pixel by the alpha of `across`
    /// times that of `down` times `strength`. `across` holds one row, which every row of the
    /// shadow repeats, and `down` one column; their first values lie on target pixel `at`.
    pub fn shade(
        &mut self,
        (across, down): (&Texture, &Texture),
        at: (i32, i32),
        strength: f32,
        clip: &[Rect],
    ) -> Result<()> {
        let programs = &self.gpu.programs;
        let program = &programs.shaded;
        let gl = self.gpu.inner.gl()?;
        // Clamped at their edges, a single row or column reaches every pixel beside it.
        let (mask_mapping, _) = mapping(down, Placement::At(at.0, at.1))?;
        let (mapping, _) = mapping(across, Placement::At(at.0, at.1))?;
        // SAFETY: `gl` made the context current; the program, its uniforms, and both textures
        // belong to it.
        unsafe {
            gl.use_program(Some(program.id));
            for (unit, texture) in [(glow::TEXTURE1, down), (glow::TEXTURE0, across)] {
                gl.active_texture(unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture.id));
                for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                    gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::NEAREST.cast_signed());
                }
            }
            gl.uniform_4_f32_slice(program.mapping.as_ref(), &mapping);
            gl.uniform_4_f32_slice(program.mask_mapping.as_ref(), &mask_mapping);
            gl.uniform_1_f32(program.opacity.as_ref(), strength);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
        self.quads(program, clip)
    }

    /// Draw the rectangles of `clip` with `program`, which is in use.
    fn quads(&mut self, program: &Program, clip: &[Rect]) -> Result<()> {
        let gl = self.gpu.inner.gl()?;
        let mut vertices = Vec::with_capacity(clip.len() * 48);
        for rect in clip.iter().filter(|rect| rect.width > 0 && rect.height > 0) {
            let (left, top) = (coordinate(rect.x)?, coordinate(rect.y)?);
            let (right, bottom) = (
                coordinate(rect.x + rect.width)?,
                coordinate(rect.y + rect.height)?,
            );
            for value in [
                left, top, right, top, left, bottom, right, top, right, bottom, left, bottom,
            ] {
                vertices.extend_from_slice(&value.to_ne_bytes());
            }
        }
        if vertices.is_empty() {
            return Ok(());
        }
        let count = i32::try_from(vertices.len() / 8)?;
        let viewport = [
            pixels(self.target.texture.width)?,
            pixels(self.target.texture.height)?,
        ];
        // SAFETY: `gl` made the context current; the buffer and program belong to it, and the
        // attribute reads two floats per vertex from the `count` vertices just uploaded.
        unsafe {
            gl.uniform_2_f32(program.viewport.as_ref(), viewport[0], viewport[1]);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.gpu.programs.vertices));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &vertices, glow::STREAM_DRAW);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.enable_vertex_attrib_array(0);
            gl.draw_arrays(glow::TRIANGLES, 0, count);
        }
        Ok(())
    }
}

/// How the vertex shader maps target pixels to `source`'s texture coordinates for
/// `placement`, and the filter that sampling needs.
fn mapping(source: &Texture, placement: Placement) -> Result<([f32; 4], u32)> {
    let size = [pixels(source.width)?, pixels(source.height)?];
    let (scale, offset, filter) = match placement {
        Placement::At(x, y) => (1.0, [-x, -y], glow::NEAREST),
        Placement::Scaled { offset, scale } => (scale, offset, glow::LINEAR),
    };
    let [from_x, from_y] = offset;
    let mapping = [
        scale / size[0],
        scale / size[1],
        coordinate(from_x)? * scale / size[0],
        coordinate(from_y)? * scale / size[1],
    ];
    Ok((mapping, filter))
}

/// A pixel coordinate as a float, exact for the 16-bit range X uses.
fn coordinate(value: i32) -> Result<f32> {
    Ok(f32::from(i16::try_from(value)?))
}

fn pixels(value: u32) -> Result<f32> {
    Ok(f32::from(u16::try_from(value)?))
}
