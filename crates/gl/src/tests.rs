use super::*;
use anyhow::{Context as _, Result, ensure};
use khronos_egl as egl;
use std::{
    ffi::c_void,
    os::fd::{FromRawFd as _, OwnedFd},
};

/// Mesa's software device, or `None` with a note when this system's EGL has none. With
/// `COMPUST_GPU_TESTS` set, as in CI, a missing device fails the test instead.
fn software() -> Result<Option<Gpu>> {
    match Gpu::open(Device::Software) {
        Ok(gpu) => Ok(Some(gpu)),
        Err(error) if std::env::var_os("COMPUST_GPU_TESTS").is_none() => {
            eprintln!("skipping: no software EGL device: {error:#}");
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn pixel(pixels: &[u8], width: i32, (x, y): (i32, i32)) -> Result<[u8; 4]> {
    let index = usize::try_from((y * width + x) * 4)?;
    let bytes = pixels
        .get(index..index + 4)
        .context("pixel outside the read")?;
    Ok(bytes.try_into()?)
}

/// A target filled with one color per listed rectangle.
fn painted(gpu: &Gpu, size: (u32, u32), fills: &[([f32; 4], Rect)]) -> Result<Target> {
    let target = gpu.target(size.0, size.1)?;
    let mut frame = gpu.frame(&target)?;
    for (color, area) in fills {
        frame.fill(*color, &[*area])?;
    }
    Ok(target)
}

#[test]
fn fills_and_composites_at_an_opacity_from_the_top_left() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];
    let source = painted(&gpu, (2, 2), &[(green, rect(0, 0, 2, 2))])?;
    // The top row is blue, so a flipped target would show it at the bottom.
    let target = painted(
        &gpu,
        (4, 4),
        &[
            (red, rect(0, 0, 4, 4)),
            ([0.0, 0.0, 1.0, 1.0], rect(0, 0, 4, 1)),
        ],
    )?;
    gpu.frame(&target)?.draw(
        source.texture(),
        Placement::At(1, 1),
        Mode::Over(0.5),
        &[rect(1, 1, 2, 2)],
    )?;
    gpu.flush()?;
    let pixels = gpu.read(&target, rect(0, 0, 4, 4))?;
    assert_eq!(pixel(&pixels, 4, (0, 0))?, [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 4, (3, 3))?, [255, 0, 0, 255]);
    let [r, g, b, a] = pixel(&pixels, 4, (2, 2))?;
    assert!(
        (127..=128).contains(&r) && (127..=128).contains(&g),
        "{r} {g}"
    );
    assert_eq!((b, a), (0, 255));
    assert_eq!(pixel(&pixels, 4, (3, 1))?, [255, 0, 0, 255]);
    Ok(())
}

#[test]
fn scaled_draws_sample_between_texels_as_the_blur_pyramid_does() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let white = [1.0; 4];
    let black = [0.0, 0.0, 0.0, 1.0];
    let stripes = painted(
        &gpu,
        (4, 1),
        &[
            (black, rect(0, 0, 4, 1)),
            (white, rect(1, 0, 1, 1)),
            (white, rect(3, 0, 1, 1)),
        ],
    )?;
    // Halving: target pixel p reads source pixel 2p + 1, midway between two texels.
    let half = gpu.target(2, 1)?;
    gpu.frame(&half)?.draw(
        stripes.texture(),
        Placement::Scaled {
            offset: [0, 0],
            scale: 2.0,
        },
        Mode::Replace,
        &[rect(0, 0, 2, 1)],
    )?;
    // Doubling: target pixel p reads source pixel (p + 0.5) / 2.
    let double = gpu.target(8, 1)?;
    gpu.frame(&double)?.draw(
        stripes.texture(),
        Placement::Scaled {
            offset: [0, 0],
            scale: 0.5,
        },
        Mode::Replace,
        &[rect(0, 0, 8, 1)],
    )?;
    gpu.flush()?;
    let halved = gpu.read(&half, rect(0, 0, 2, 1))?;
    for x in 0..2 {
        let [r, ..] = pixel(&halved, 2, (x, 0))?;
        assert!((127..=128).contains(&r), "halved {x}: {r}");
    }
    let doubled = gpu.read(&double, rect(0, 0, 8, 1))?;
    let reds: Vec<u8> = (0..8)
        .map(|x| pixel(&doubled, 8, (x, 0)).map(|[r, ..]| r))
        .collect::<Result<_>>()?;
    // Pixel 0 reads 0.25, clamped to the first texel; pixel 1 reads 0.75, a quarter of the way
    // to the white second texel; pixel 2 reads 1.25, three quarters of the way.
    assert_eq!(reds.first(), Some(&0));
    assert!(
        reds.get(1).is_some_and(|r| (63..=64).contains(r)),
        "{reds:?}"
    );
    assert!(
        reds.get(2).is_some_and(|r| (191..=192).contains(r)),
        "{reds:?}"
    );
    Ok(())
}

type ExportQuery = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut i32,
    *mut i32,
    *mut u64,
) -> egl::Boolean;
type Export = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut i32,
    *mut i32,
    *mut i32,
) -> egl::Boolean;

/// Share `target` as a dma-buf through `EGL_MESA_image_dma_buf_export`, as the X server
/// shares a pixmap.
fn export(gpu: &Gpu, target: &Target) -> Result<Dmabuf> {
    let inner = &gpu.inner;
    // SAFETY: EGL_MESA_image_dma_buf_export defines these functions with these signatures.
    let query: ExportQuery =
        unsafe { device::extension(&inner.egl, "eglExportDMABUFImageQueryMESA") }?;
    // SAFETY: as above.
    let export: Export = unsafe { device::extension(&inner.egl, "eglExportDMABUFImageMESA") }?;
    let name = target.texture.id.0.get();
    // SAFETY: EGL_GL_TEXTURE_2D images take the texture's name, cast to a pointer, as their
    // client buffer.
    let client = unsafe { egl::ClientBuffer::from_ptr(name as usize as *mut c_void) };
    let image = inner.egl.create_image(
        inner.display,
        inner.context,
        0x30B1,
        client,
        &[egl::ATTRIB_NONE],
    )?;
    let (mut format, mut planes, mut modifier) = (0, 0, 0);
    // SAFETY: the image is live, and the query writes one value to each pointer.
    let queried = unsafe {
        query(
            inner.display.as_ptr(),
            image.as_ptr(),
            &raw mut format,
            &raw mut planes,
            &raw mut modifier,
        )
    };
    ensure!(
        queried == egl::TRUE && planes == 1,
        "cannot export the texture"
    );
    let (mut fd, mut stride, mut offset) = (-1, 0, 0);
    // SAFETY: the image has one plane, so EGL writes one value to each pointer.
    let exported = unsafe {
        export(
            inner.display.as_ptr(),
            image.as_ptr(),
            &raw mut fd,
            &raw mut stride,
            &raw mut offset,
        )
    };
    let _destroyed = inner.egl.destroy_image(inner.display, image);
    ensure!(
        exported == egl::TRUE && fd >= 0,
        "cannot export the texture"
    );
    // SAFETY: EGL hands over a new file descriptor that nothing else owns.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    Ok(Dmabuf {
        width: target.texture.width,
        height: target.texture.height,
        format: u32::try_from(format)?,
        modifier,
        planes: vec![Plane {
            fd,
            offset: u32::try_from(offset)?,
            stride: u32::try_from(stride)?,
        }],
    })
}

#[test]
fn imported_dma_bufs_show_their_buffer() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let original = painted(
        &gpu,
        (4, 2),
        &[
            ([0.0, 0.0, 1.0, 1.0], rect(0, 0, 4, 2)),
            ([1.0, 1.0, 0.0, 1.0], rect(2, 1, 2, 1)),
        ],
    )?;
    gpu.flush()?;
    // Exporting is only how the test makes a dma-buf; drivers that cannot leave the import
    // to be tested on hardware.
    let buffer = match export(&gpu, &original) {
        Ok(buffer) => buffer,
        Err(error) => {
            eprintln!("skipping: {error:#}");
            return Ok(());
        }
    };
    let imported = gpu.import(&buffer)?;
    drop(buffer);
    let copy = gpu.target(4, 2)?;
    gpu.frame(&copy)?.draw(
        &imported,
        Placement::At(0, 0),
        Mode::Replace,
        &[rect(0, 0, 4, 2)],
    )?;
    gpu.flush()?;
    let pixels = gpu.read(&copy, rect(0, 0, 4, 2))?;
    assert_eq!(pixel(&pixels, 4, (0, 0))?, [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 4, (3, 1))?, [255, 255, 0, 255]);
    Ok(())
}

#[test]
fn a_render_node_opens_its_gpu() -> Result<()> {
    use std::os::unix::fs::MetadataExt as _;
    let Some(node) = std::fs::read_dir("/dev/dri")
        .into_iter()
        .flatten()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
    else {
        eprintln!("skipping: no DRM render node");
        return Ok(());
    };
    let number = node.metadata()?.rdev();
    let gpu = Gpu::open(Device::Drm(number))
        .with_context(|| format!("opening {}", node.path().display()))?;
    eprintln!("{}: {}", node.path().display(), gpu.renderer()?);
    let target = painted(&gpu, (2, 2), &[([0.0, 1.0, 0.0, 1.0], rect(0, 0, 2, 2))])?;
    gpu.flush()?;
    let pixels = gpu.read(&target, rect(0, 0, 2, 2))?;
    assert_eq!(pixel(&pixels, 2, (1, 1))?, [0, 255, 0, 255]);
    Ok(())
}

#[test]
fn two_contexts_on_one_thread_keep_their_own_objects() -> Result<()> {
    let full = rect(0, 0, 2, 2);
    let Some(first) = software()? else {
        return Ok(());
    };
    let kept = painted(&first, (2, 2), &[([0.0, 1.0, 0.0, 1.0], full)])?;
    // Opening the second context makes it current, and its objects take the same names as
    // the first's, each in its own context.
    let Some(second) = software()? else {
        return Ok(());
    };
    let other = painted(&second, (2, 2), &[([1.0, 0.0, 0.0, 1.0], full)])?;
    drop(kept);
    second.flush()?;
    assert_eq!(
        pixel(&second.read(&other, full)?, 2, (1, 1))?,
        [255, 0, 0, 255]
    );
    let again = painted(&first, (2, 2), &[([0.0, 0.0, 1.0, 1.0], full)])?;
    assert_eq!(
        pixel(&first.read(&again, full)?, 2, (0, 0))?,
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&second.read(&other, full)?, 2, (0, 0))?,
        [255, 0, 0, 255]
    );
    Ok(())
}

#[test]
fn repeated_textures_tile() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let tile = painted(
        &gpu,
        (2, 1),
        &[
            ([1.0, 0.0, 0.0, 1.0], rect(0, 0, 1, 1)),
            ([0.0, 0.0, 1.0, 1.0], rect(1, 0, 1, 1)),
        ],
    )?;
    tile.texture().repeat()?;
    let wide = gpu.target(5, 1)?;
    gpu.frame(&wide)?.draw(
        tile.texture(),
        Placement::At(0, 0),
        Mode::Replace,
        &[rect(0, 0, 5, 1)],
    )?;
    let pixels = gpu.read(&wide, rect(0, 0, 5, 1))?;
    let reds: Vec<u8> = (0..5)
        .map(|x| pixel(&pixels, 5, (x, 0)).map(|[r, ..]| r))
        .collect::<Result<_>>()?;
    assert_eq!(reds, [255, 0, 255, 0, 255]);
    Ok(())
}
