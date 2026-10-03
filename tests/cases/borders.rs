use crate::{rect, support::Desktop};
use anyhow::Result;
use x11rb::protocol::{
    shape::{ConnectionExt as _, SK, SO},
    xproto::{
        ChangeWindowAttributesAux, ClipOrdering, ConfigureWindowAux, ConnectionExt, Rectangle,
    },
};

fn bordered_window(desktop: &Desktop) -> Result<u32> {
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0x0000_ff00),
        )?
        .check()?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().border_width(4))?
        .check()?;
    Ok(window)
}

#[test]
fn repaints_all_border_edges_when_focus_color_changes() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = bordered_window(&desktop)?;
    desktop.map(window)?;
    desktop.until_pixel((60, 60), |pixel| pixel == [255, 0, 0])?;

    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0x0000_00ff),
        )?
        .check()?;

    desktop.until_pixel((22, 60), |pixel| pixel == [0, 0, 255])?;
    for point in [(22, 60), (60, 22), (126, 60), (60, 126)] {
        assert_eq!(desktop.pixel(point)?, [0, 0, 255], "border at {point:?}");
    }
    assert_eq!(desktop.pixel((60, 60))?, [255, 0, 0]);
    desktop.screenshot("borders-focus-color")?;
    Ok(())
}

#[test]
fn recaptures_all_border_edges_after_resize() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\nvsync = false")?;
    let window = bordered_window(&desktop)?;
    desktop.map(window)?;
    desktop.until_pixel((60, 60), |pixel| pixel == [255, 0, 0])?;

    desktop
        .conn
        .configure_window(
            window,
            &ConfigureWindowAux::new()
                .width(140)
                .height(120)
                .border_width(6),
        )?
        .check()?;

    desktop.until_pixel((170, 60), |pixel| pixel == [0, 255, 0])?;
    for point in [(22, 60), (60, 22), (170, 60), (60, 150)] {
        assert_eq!(desktop.pixel(point)?, [0, 255, 0], "border at {point:?}");
    }
    assert_eq!(desktop.pixel((166, 60))?, [0, 255, 0]);
    assert_eq!(desktop.pixel((172, 60))?, [24, 24, 32]);
    desktop.screenshot("borders-resized")?;
    Ok(())
}

#[test]
fn preserves_client_shape_and_restores_borders_when_shape_is_removed() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0")?;
    let window = bordered_window(&desktop)?;
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
                x: -4,
                y: -4,
                width: 48,
                height: 108,
            }],
        )?
        .check()?;
    desktop.map(window)?;
    desktop.until_pixel((60, 60), |pixel| pixel == [255, 0, 0])?;
    assert_eq!(desktop.pixel((90, 60))?, [24, 24, 32]);
    assert_eq!(desktop.pixel((126, 60))?, [24, 24, 32]);
    desktop.screenshot("borders-client-shape")?;

    desktop
        .conn
        .shape_mask(SO::SET, SK::BOUNDING, window, 0, 0, x11rb::NONE)?
        .check()?;

    desktop.until_pixel((90, 60), |pixel| pixel == [255, 0, 0])?;
    for point in [(22, 60), (60, 22), (126, 60), (60, 126)] {
        assert_eq!(desktop.pixel(point)?, [0, 255, 0], "border at {point:?}");
    }
    desktop.screenshot("borders-shape-removed")?;
    Ok(())
}
