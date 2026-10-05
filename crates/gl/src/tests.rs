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
fn bordered_corner_draws_content_and_samples_the_top_left_border_pixel() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let source = painted(
        &gpu,
        (4, 4),
        &[
            ([0.0, 1.0, 0.0, 1.0], rect(0, 0, 4, 4)),
            ([0.0, 0.0, 1.0, 1.0], rect(0, 0, 1, 1)),
            ([1.0, 0.0, 0.0, 1.0], rect(1, 1, 3, 3)),
        ],
    )?;
    let inner = gpu.alpha(2, 2, &[0, 0, 0, 255])?;
    let ring = gpu.alpha(2, 2, &[0, 128, 255, 0])?;
    let target = painted(&gpu, (4, 4), &[([1.0; 4], rect(0, 0, 4, 4))])?;
    gpu.frame(&target)?.draw_bordered_covered(
        source.texture(),
        Placement::At(0, 0),
        (&inner, (0, 0)),
        (&ring, (0, 0)),
        0.5,
        &[rect(0, 0, 2, 2)],
    )?;
    gpu.flush()?;
    let pixels = gpu.read(&target, rect(0, 0, 4, 4))?;
    for (point, expected) in [
        ((0, 0), [255_u8, 255, 255, 255]),
        ((1, 0), [191, 191, 255, 255]),
        ((0, 1), [127, 127, 255, 255]),
        ((1, 1), [255, 127, 127, 255]),
        ((2, 2), [255, 255, 255, 255]),
    ] {
        let shown = pixel(&pixels, 4, point)?;
        for (channel, (shown, expected)) in shown.iter().zip(expected).enumerate() {
            assert!(
                i16::from(*shown).abs_diff(i16::from(expected)) <= 1,
                "{point:?} channel {channel}: {shown} != {expected}"
            );
        }
    }
    Ok(())
}

#[test]
fn shadows_darken_by_the_product_of_two_profiles() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    shades(&gpu)
}

#[test]
fn a_render_node_draws_shadows_as_the_software_device_does() -> Result<()> {
    let Some(gpu) = hardware()? else {
        return Ok(());
    };
    shades(&gpu)
}

/// Given a white target and a shadow three pixels wide and two high at (2, 1), each pixel
/// darkens by its column's value times its row's times the strength, and only in the clip.
fn shades(gpu: &Gpu) -> Result<()> {
    let target = painted(gpu, (6, 5), &[([1.0; 4], rect(0, 0, 6, 5))])?;
    let across = gpu.alpha(3, 1, &[255, 128, 0])?;
    let down = gpu.alpha(1, 2, &[255, 64])?;
    gpu.frame(&target)?
        .shade((&across, &down), (2, 1), 0.5, &[rect(2, 1, 3, 2)])?;
    gpu.flush()?;
    let pixels = gpu.read(&target, rect(0, 0, 6, 5))?;
    for (point, expected) in [
        ((2, 1), 127..=128),
        ((3, 1), 190..=192),
        ((4, 1), 255..=255),
        ((2, 2), 222..=224),
        ((3, 2), 238..=240),
        ((1, 1), 255..=255),
        ((2, 0), 255..=255),
        ((2, 3), 255..=255),
        ((5, 2), 255..=255),
    ] {
        let [r, g, b, a] = pixel(&pixels, 6, point)?;
        assert!(
            expected.contains(&r) && r == g && g == b,
            "{point:?}: {r} {g} {b}"
        );
        assert_eq!(a, 255, "{point:?}");
    }
    assert!(gpu.alpha(3, 2, &[0; 5]).is_err());
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

/// The GPU behind this machine's first DRM render node, or `None` with a note without one.
fn hardware() -> Result<Option<Gpu>> {
    use std::os::unix::fs::MetadataExt as _;
    let Some(node) = std::fs::read_dir("/dev/dri")
        .into_iter()
        .flatten()
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
    else {
        eprintln!("skipping: no DRM render node");
        return Ok(None);
    };
    let number = node.metadata()?.rdev();
    let gpu = Gpu::open(Device::Drm(number))
        .with_context(|| format!("opening {}", node.path().display()))?;
    eprintln!("{}: {}", node.path().display(), gpu.renderer()?);
    Ok(Some(gpu))
}

#[test]
fn a_render_node_opens_its_gpu() -> Result<()> {
    let Some(gpu) = hardware()? else {
        return Ok(());
    };
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

#[test]
fn masked_draws_weigh_the_source_by_the_mask_alpha_and_opacity() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let red = painted(&gpu, (3, 1), &[([1.0, 0.0, 0.0, 1.0], rect(0, 0, 3, 1))])?;
    // A transparent, a half-transparent, and an opaque mask pixel.
    let mask = painted(
        &gpu,
        (3, 1),
        &[
            ([0.0; 4], rect(0, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 0.5], rect(1, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 1.0], rect(2, 0, 1, 1)),
        ],
    )?;
    let target = painted(&gpu, (3, 2), &[([0.0, 0.0, 1.0, 1.0], rect(0, 0, 3, 2))])?;
    let mut frame = gpu.frame(&target)?;
    frame.draw_masked(
        red.texture(),
        Placement::At(0, 0),
        (mask.texture(), (0, 0)),
        1.0,
        &[rect(0, 0, 3, 1)],
    )?;
    // The second row places the source and the mask a row lower, at half opacity.
    frame.draw_masked(
        red.texture(),
        Placement::At(0, 1),
        (mask.texture(), (0, 1)),
        0.5,
        &[rect(0, 1, 3, 1)],
    )?;
    let pixels = gpu.read(&target, rect(0, 0, 3, 2))?;
    let near = |[r, g, b, _]: [u8; 4], [er, eb]: [u8; 2]| {
        r.abs_diff(er) <= 1 && g == 0 && b.abs_diff(eb) <= 1
    };
    for (point, expected) in [
        ((0, 0), [0, 255]),
        ((1, 0), [128, 127]),
        ((2, 0), [255, 0]),
        ((0, 1), [0, 255]),
        ((1, 1), [64, 191]),
        ((2, 1), [128, 127]),
    ] {
        let found = pixel(&pixels, 3, point)?;
        assert!(near(found, expected), "{point:?}: {found:?}");
    }
    Ok(())
}

#[test]
fn covered_draws_keep_the_source_alpha_and_weigh_it_by_the_mask() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    // Half-transparent red, premultiplied, through a transparent, a half, and an opaque mask
    // pixel, over blue.
    let red = painted(&gpu, (3, 1), &[([0.5, 0.0, 0.0, 0.5], rect(0, 0, 3, 1))])?;
    let mask = painted(
        &gpu,
        (3, 1),
        &[
            ([0.0; 4], rect(0, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 0.5], rect(1, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 1.0], rect(2, 0, 1, 1)),
        ],
    )?;
    let target = painted(&gpu, (3, 1), &[([0.0, 0.0, 1.0, 1.0], rect(0, 0, 3, 1))])?;
    gpu.frame(&target)?.draw_covered(
        red.texture(),
        Placement::At(0, 0),
        (mask.texture(), (0, 0)),
        1.0,
        &[rect(0, 0, 3, 1)],
    )?;
    let pixels = gpu.read(&target, rect(0, 0, 3, 1))?;
    for (x, [er, eb]) in [(0, [0_u8, 255_u8]), (1, [64, 191]), (2, [128, 128])] {
        let [r, g, b, _] = pixel(&pixels, 3, (x, 0))?;
        assert!(
            r.abs_diff(er) <= 1 && g == 0 && b.abs_diff(eb) <= 1,
            "{x}: {r} {g} {b}"
        );
    }
    Ok(())
}

#[test]
fn masked_covered_draws_weigh_the_source_by_both_alphas() -> Result<()> {
    let Some(gpu) = software()? else {
        return Ok(());
    };
    let red = painted(&gpu, (2, 2), &[([1.0, 0.0, 0.0, 1.0], rect(0, 0, 2, 2))])?;
    let mask = painted(
        &gpu,
        (2, 1),
        &[
            ([0.0, 0.0, 0.0, 1.0], rect(0, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 0.5], rect(1, 0, 1, 1)),
        ],
    )?;
    let cover = painted(
        &gpu,
        (2, 1),
        &[
            ([0.0, 0.0, 0.0, 0.5], rect(0, 0, 1, 1)),
            ([0.0, 0.0, 0.0, 1.0], rect(1, 0, 1, 1)),
        ],
    )?;
    let target = painted(&gpu, (2, 2), &[([0.0, 0.0, 1.0, 1.0], rect(0, 0, 2, 2))])?;
    let mut frame = gpu.frame(&target)?;
    // Both pixels weigh one half: 1 × 0.5 and 0.5 × 1.
    frame.draw_masked_covered(
        red.texture(),
        Placement::At(0, 0),
        (mask.texture(), (0, 0)),
        (cover.texture(), (0, 0)),
        1.0,
        &[rect(0, 0, 2, 1)],
    )?;
    // A row lower, the cover moves one pixel left: 1 × 1 and 0.5 × (clamped) 1, at half
    // opacity.
    frame.draw_masked_covered(
        red.texture(),
        Placement::At(0, 1),
        (mask.texture(), (0, 1)),
        (cover.texture(), (-1, 1)),
        0.5,
        &[rect(0, 1, 2, 1)],
    )?;
    let pixels = gpu.read(&target, rect(0, 0, 2, 2))?;
    for (point, [er, eb]) in [
        ((0, 0), [128_u8, 127_u8]),
        ((1, 0), [128, 127]),
        ((0, 1), [128, 127]),
        ((1, 1), [64, 191]),
    ] {
        let [r, g, b, _] = pixel(&pixels, 2, point)?;
        assert!(
            r.abs_diff(er) <= 1 && g == 0 && b.abs_diff(eb) <= 1,
            "{point:?}: {r} {g} {b}"
        );
    }
    Ok(())
}
