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
varying vec2 texcoord;
void main() {
    texcoord = position * mapping.xy + mapping.zw;
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

struct Program {
    id: glow::Program,
    viewport: Option<Location>,
    mapping: Option<Location>,
    opacity: Option<Location>,
    color: Option<Location>,
}

/// The shaders and vertex buffer every draw uses.
pub(crate) struct Programs {
    inner: Rc<Inner>,
    textured: Program,
    solid: Program,
    vertices: glow::Buffer,
}

impl Programs {
    pub(crate) fn new(inner: &Rc<Inner>) -> Result<Self> {
        let textured = program(inner, TEXTURED)?;
        let solid = program(inner, SOLID)?;
        let gl = inner.gl()?;
        // SAFETY: `gl` made the context current.
        let vertices = unsafe { gl.create_buffer() }.map_err(anyhow::Error::msg)?;
        Ok(Self {
            inner: Rc::clone(inner),
            textured,
            solid,
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
        Ok(Program {
            id,
            viewport: gl.get_uniform_location(id, "viewport"),
            mapping: gl.get_uniform_location(id, "mapping"),
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
    /// the X server presenting it, then sees them.
    pub fn flush(&self) -> Result<()> {
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current, and Flush takes no arguments.
        unsafe { gl.flush() };
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

/// A pixel coordinate as a float, exact for the 16-bit range X uses.
fn coordinate(value: i32) -> Result<f32> {
    Ok(f32::from(i16::try_from(value)?))
}

fn pixels(value: u32) -> Result<f32> {
    Ok(f32::from(u16::try_from(value)?))
}
