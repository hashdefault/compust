use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::{
    shape::{ConnectionExt as _, SK, SO},
    xproto::*,
};

#[test]
fn clips_extreme_shape_coordinates_before_translation() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(-40, -40), 0x00ff_0000)?;
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
                x: -32760,
                y: -32760,
                width: 32810,
                height: 32810,
            }],
        )?
        .check()?;

    desktop.map(window)?;

    desktop.until_pixel((5, 5), |p| p == [255, 0, 0])?;
    assert_eq!(desktop.pixel((20, 5))?, [24, 24, 32]);
    assert_eq!(desktop.pixel((5, 20))?, [24, 24, 32]);
    desktop.screenshot("shape-extreme-offscreen")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn restores_disjoint_shape_after_empty_region() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((30, 30), |p| p == [255, 0, 0])?;

    for shape in [
        Vec::new(),
        vec![
            Rectangle {
                x: 0,
                y: 0,
                width: 20,
                height: 100,
            },
            Rectangle {
                x: 80,
                y: 0,
                width: 20,
                height: 100,
            },
        ],
    ] {
        desktop
            .conn
            .shape_rectangles(
                SO::SET,
                SK::BOUNDING,
                ClipOrdering::UNSORTED,
                window,
                0,
                0,
                &shape,
            )?
            .check()?;
        let expected = if shape.is_empty() {
            [24, 24, 32]
        } else {
            [255, 0, 0]
        };
        desktop.until_pixel((30, 30), |p| p == expected)?;
        assert_eq!(desktop.pixel((70, 30))?, [24, 24, 32]);
        assert_eq!(desktop.pixel((110, 30))?, expected);
    }

    desktop.screenshot("shape-disjoint-restored")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn tracks_bordered_window_across_screen_edges() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\nvsync = false")?;
    let window = desktop.window(rect(-10, -10), 0x00ff_0000)?;
    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0x0000_ff00),
        )?
        .check()?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().border_width(20))?
        .check()?;
    desktop.map(window)?;
    desktop.until_pixel((5, 5), |p| p == [0, 255, 0])?;
    desktop.until_pixel((15, 15), |p| p == [255, 0, 0])?;
    desktop.screenshot("shape-border-all-edges")?;
    for point in [(5, 5), (115, 15), (15, 115), (115, 115)] {
        assert_eq!(
            desktop.pixel(point)?,
            [0, 255, 0],
            "border missing at {point:?}"
        );
    }

    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(300).y(220))?
        .check()?;

    desktop.until_pixel((15, 15), |p| p == [24, 24, 32])?;
    desktop.until_pixel((310, 230), |p| p == [0, 255, 0])?;
    desktop.screenshot("shape-border-offscreen")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn clips_blur_to_offscreen_window_shape() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 4")?;
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
    let window = desktop.window(rect(-40, 40), 0)?;
    desktop.opacity(window, 0x8000_0000)?;
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
                x: 40,
                y: 0,
                width: 20,
                height: 100,
            }],
        )?
        .check()?;

    desktop.map(window)?;

    desktop.until_pixel((11, 80), |[r, g, b]| {
        (40..90).contains(&r) && r == g && g == b
    })?;
    assert_eq!(desktop.pixel((21, 80))?, [255, 255, 255]);
    assert_eq!(desktop.pixel((41, 80))?, [255, 255, 255]);
    desktop.screenshot("shape-offscreen-blur")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn restores_large_window_after_moving_fully_offscreen() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = desktop.window(
        Rectangle {
            x: -2000,
            y: -1000,
            width: 4096,
            height: 2048,
        },
        0x00ff_0000,
    )?;
    desktop.map(window)?;
    desktop.until_pixel((300, 200), |p| p == [255, 0, 0])?;

    for (x, expected) in [(320, [24, 24, 32]), (-2000, [255, 0, 0])] {
        desktop
            .conn
            .configure_window(window, &ConfigureWindowAux::new().x(x))?
            .check()?;
        desktop.until_pixel((300, 200), |p| p == expected)?;
        assert_eq!(desktop.pixel((0, 0))?, expected);
    }

    desktop.screenshot("shape-large-restored")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
