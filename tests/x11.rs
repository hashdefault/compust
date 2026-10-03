#[path = "cases/capture_races.rs"]
mod capture_races;
#[path = "cases/client_lifecycle.rs"]
mod client_lifecycle;
#[path = "cases/compat.rs"]
mod compat;
#[path = "cases/destruction.rs"]
mod destruction;
#[path = "cases/fades.rs"]
mod fades;
#[path = "cases/properties.rs"]
mod properties;
#[path = "cases/shapes.rs"]
mod shapes;
#[path = "cases/stability.rs"]
mod stability;
mod support;

#[path = "cases/resources.rs"]
mod resources;

#[path = "cases/monitors.rs"]
mod monitors;

#[path = "cases/presentation.rs"]
mod presentation;
use anyhow::Result;
use support::Desktop;
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::*,
    },
};

fn rect(x: i16, y: i16) -> Rectangle {
    Rectangle {
        x,
        y,
        width: 100,
        height: 100,
    }
}

#[test]
fn composites_opacity_moves_resizes_and_destruction() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let back = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(back)?;
    desktop.until_pixel((50, 50), |p| p == [255, 0, 0])?;
    let front = desktop.window(rect(40, 40), 0x0000_00ff)?;
    desktop.opacity(front, 0x8000_0000)?;
    desktop.map(front)?;
    desktop.until_pixel((50, 50), |[r, g, b]| {
        (126..=129).contains(&r) && g == 0 && (126..=129).contains(&b)
    })?;
    desktop.screenshot("transparency")?;
    desktop
        .conn
        .configure_window(front, &ConfigureWindowAux::new().x(160).y(40).width(120))?
        .check()?;
    desktop.until_pixel((50, 50), |p| p == [255, 0, 0])?;
    desktop.until_pixel((270, 60), |[_, _, b]| b > 120)?;
    desktop.conn.destroy_window(front)?.check()?;
    desktop.until_pixel((270, 60), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn blurs_background_behind_translucent_window() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 4")?;
    let background = desktop.window(
        Rectangle {
            x: 0,
            y: 0,
            width: 320,
            height: 240,
        },
        0x00ff_ffff,
    )?;
    desktop.map(background)?;
    let gc = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_gc(gc, background, &CreateGCAux::new().foreground(0))?
        .check()?;
    let stripes: Vec<_> = (0_i16..320)
        .step_by(2)
        .map(|x| Rectangle {
            x,
            y: 0,
            width: 1,
            height: 240,
        })
        .collect();
    desktop
        .conn
        .poly_fill_rectangle(background, gc, &stripes)?
        .check()?;
    let front = desktop.window(rect(100, 60), 0)?;
    desktop.opacity(front, 0x8000_0000)?;
    desktop.map(front)?;
    desktop.until_pixel((140, 100), |[r, g, b]| {
        (40..=90).contains(&r) && r == g && g == b
    })?;
    desktop.screenshot("blur")?;
    Ok(())
}

#[test]
fn blur_spreads_edges_symmetrically_without_shifting_them() -> Result<()> {
    edge_blur(4, 20)?;
    edge_blur(16, 60)
}

/// Blur a black/white edge at x = 160, aligned with every pyramid level, and check that
/// the edge spreads by about the radius, stays centered, and leaves distant pixels alone.
fn edge_blur(radius: u8, span: i16) -> Result<()> {
    let mut desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = {radius}"))?;
    let dark = Rectangle {
        x: 0,
        y: 0,
        width: 160,
        height: 240,
    };
    let light = Rectangle { x: 160, ..dark };
    desktop.map(desktop.window(dark, 0)?)?;
    desktop.map(desktop.window(light, 0x00ff_ffff)?)?;
    desktop.until_pixel((160, 120), |pixel| pixel == [255, 255, 255])?;
    // A nearly transparent window shows the blurred background through its 1/255 opacity.
    let front = desktop.window(
        Rectangle {
            x: 160 - span - 4,
            y: 60,
            width: (2 * span + 8).unsigned_abs(),
            height: 120,
        },
        0,
    )?;
    desktop.opacity(front, 0x0101_0101)?;
    desktop.map(front)?;
    desktop.until_pixel((159, 120), |[r, _, _]| (10..245).contains(&r))?;
    let row = (160 - span..160 + span)
        .map(|x| desktop.pixel((x, 120)).map(|[r, _, _]| r))
        .collect::<Result<Vec<_>>>()?;
    assert!(
        row.windows(2)
            .all(|pair| matches!(pair, [left, right] if left <= right)),
        "radius {radius}: {row:?}"
    );
    let (dark_half, light_half) = row.split_at(usize::try_from(span)?);
    for (dark, light) in dark_half.iter().rev().zip(light_half) {
        let sum = u16::from(*dark) + u16::from(*light);
        assert!((250..=256).contains(&sum), "radius {radius}: {row:?}");
    }
    assert!(
        row.first().is_some_and(|&first| first <= 2) && row.last().is_some_and(|&last| last >= 250),
        "radius {radius}: {row:?}"
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn fades_open_and_closed_with_exact_endpoints() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 400\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |[r, _, _]| (70..200).contains(&r))?;
    desktop.screenshot("fade")?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.conn.destroy_window(window)?.check()?;
    desktop.until_pixel((40, 40), |[r, _, _]| (70..200).contains(&r))?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    Ok(())
}

#[test]
fn respects_bounding_shape_and_xrender_fallback() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\nvsync = false")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop
        .conn
        .shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            window,
            0,
            0,
            &[Rectangle {
                x: 0,
                y: 0,
                width: 30,
                height: 100,
            }],
        )?
        .check()?;
    desktop.map(window)?;
    desktop.until_pixel((30, 40), |p| p == [255, 0, 0])?;
    assert_eq!(desktop.pixel((70, 40))?, [24, 24, 32]);
    desktop.screenshot("shape")?;
    Ok(())
}

#[test]
fn refuses_second_compositor_and_diagnoses_without_claiming() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_compust"))
        .args(["--display", &desktop.display])
        .output()?;
    assert!(!result.status.success());
    assert!(desktop.compositor.0.try_wait()?.is_none());
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_compust"))
        .args(["--display", &desktop.display, "--diagnose"])
        .output()?;
    assert!(result.status.success());
    Ok(())
}
