use super::scene::{RED, create, near, wait_state};
use crate::{Args, surface::Surface};
use anyhow::{Context, Result, ensure};
use std::{fs::File, io::Write, path::Path};
use x11rb::protocol::xproto::{ConnectionExt as _, Rectangle};

// Golden cumulative coverage of three width-9 box filters, normalized by 9^3. The
// radius-12 shadow uses this ramp at each edge of a 100-pixel rectangle. This is
// independent of the compositor's profile implementation and its renderer.
const EDGE: [u8; 25] = [
    0, 1, 3, 7, 12, 20, 29, 42, 58, 76, 96, 117, 138, 159, 179, 197, 213, 226, 235, 243, 248, 252,
    254, 255, 255,
];

pub(super) fn check(surface: &Surface, args: &Args) -> Result<()> {
    let window = create(
        surface,
        Rectangle {
            x: 100,
            y: 60,
            width: 100,
            height: 100,
        },
        RED,
    )?;
    surface.conn.map_window(window)?.check()?;
    wait_state(surface, false)?;
    surface.until("shadow fixture", || {
        Ok(surface.pixel((150, 110))? == RED && near(surface.pixel((203, 110))?, 0x0075_7575))
    })?;
    snapshot(surface, args, "shadows")?;
    let bytes = image(&args.output.join("shadows.ppm"), surface)?;
    let mut maximum = 0_u8;
    let mut differing = 0_u64;
    for (index, actual) in bytes.chunks_exact(3).enumerate() {
        let x = i32::try_from(index % usize::from(surface.width))?;
        let y = i32::try_from(index / usize::from(surface.width))?;
        let expected = expected(x, y)?;
        let delta = actual
            .iter()
            .zip(expected)
            .map(|(a, e)| a.abs_diff(e))
            .max()
            .unwrap_or(0);
        ensure!(
            delta <= 2,
            "shadow pixel ({x},{y}) is {actual:?}, expected {expected:?}"
        );
        maximum = maximum.max(delta);
        differing += u64::from(delta != 0);
    }
    let mut report = File::create(args.output.join("shadows.csv"))?;
    writeln!(
        report,
        "pixels_checked,differing_pixels,max_channel_delta,tolerance"
    )?;
    writeln!(report, "{},{differing},{maximum},2", bytes.len() / 3)?;
    surface.conn.destroy_window(window)?.check()?;
    println!(
        "PASS: {} shadow scene pixels, max channel delta={maximum}",
        bytes.len() / 3
    );
    Ok(())
}

fn coverage(position: i32) -> Result<u8> {
    if !(0..124).contains(&position) {
        return Ok(0);
    }
    let edge = position.min(123 - position);
    Ok(EDGE.get(usize::try_from(edge)?).copied().unwrap_or(255))
}

fn expected(x: i32, y: i32) -> Result<[u8; 3]> {
    if (100..200).contains(&x) && (60..160).contains(&y) {
        return Ok([255, 0, 0]);
    }
    let alpha = (u32::from(coverage(x - 92)?) * u32::from(coverage(y - 54)?) + 127) / 255;
    Ok([u8::try_from(255 - alpha)?; 3])
}

pub(super) fn snapshot(surface: &Surface, args: &Args, name: &str) -> Result<()> {
    let path = args.output.join(format!("{name}.ppm"));
    surface.screenshot(&path)?;
    if let Some(reference) = &args.features.reference {
        let reference = image(&reference.join(format!("{name}.ppm")), surface)?;
        let actual = image(&path, surface)?;
        let mut differing = 0_u64;
        let mut maximum = 0_u8;
        for (index, (actual, reference)) in actual
            .chunks_exact(3)
            .zip(reference.chunks_exact(3))
            .enumerate()
        {
            let delta = actual
                .iter()
                .zip(reference)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            maximum = maximum.max(delta);
            differing += u64::from(delta != 0);
            ensure!(
                delta <= 2,
                "renderer difference in {name} at pixel {index}: {actual:?} vs {reference:?}"
            );
        }
        let mut report = File::create(args.output.join(format!("{name}-comparison.csv")))?;
        writeln!(
            report,
            "pixels_compared,differing_pixels,max_channel_delta,tolerance"
        )?;
        writeln!(report, "{},{differing},{maximum},2", actual.len() / 3)?;
        println!("PASS: {name} renderer comparison, max channel delta={maximum}");
    }
    Ok(())
}

fn image(path: &Path, surface: &Surface) -> Result<Vec<u8>> {
    let header = format!("P6\n{} {}\n255\n", surface.width, surface.height);
    let image = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let pixels = image
        .strip_prefix(header.as_bytes())
        .context("reference PPM geometry or format differs")?;
    ensure!(
        pixels.len() == usize::from(surface.width) * usize::from(surface.height) * 3,
        "invalid PPM length"
    );
    Ok(pixels.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_has_red_interior_and_offset_shadow_edges() -> Result<()> {
        // Given the fixed red rectangle and white background, sample known edge values.
        // When evaluating pixels beside its moved rectangle, the shadow follows both axes.
        for (point, value) in [
            ((200, 110), 58),
            ((203, 110), 117),
            ((150, 160), 29),
            ((205, 165), 203),
            ((216, 110), 255),
        ] {
            // Then the independent golden samples match the mathematical box-filter result.
            assert_eq!(expected(point.0, point.1)?, [value; 3]);
        }
        assert_eq!(expected(150, 110)?, [255, 0, 0]);
        Ok(())
    }
}
