use super::{
    scene::{RED, WHITE, create, typed},
    shadows::snapshot,
};
use crate::{Args, surface::Surface};
use anyhow::{Result, ensure};
use x11rb::{
    connection::Connection as _,
    protocol::xproto::{AtomEnum, ConnectionExt as _, CreateGCAux, PropMode, Rectangle},
    wrapper::ConnectionExt as _,
};

/// The class the feature configuration rounds with corners of radius 13.
const CLASS: &[u8] = b"compust-feature\0CompustFeatureCorners\0";

/// Check rounded corners: an opaque window and a half-transparent one over a checkered
/// pattern, both rounded by the configuration's rule, with their rounded shadows and the
/// blur behind the second. With a reference, the image must match the other painter's.
pub(super) fn check(surface: &Surface, args: &Args) -> Result<()> {
    let checker = create(
        surface,
        Rectangle {
            x: 150,
            y: 10,
            width: 150,
            height: 140,
        },
        WHITE,
    )?;
    typed(surface, checker, "_NET_WM_WINDOW_TYPE_DESKTOP")?;
    surface.conn.map_window(checker)?.check()?;
    let gc = surface.conn.generate_id()?;
    surface
        .conn
        .create_gc(gc, checker, &CreateGCAux::new().foreground(0))?
        .check()?;
    let squares: Vec<_> = (0_i16..18)
        .flat_map(|row| {
            (0_i16..19)
                .filter(move |column| (row + column) % 2 == 0)
                .map(move |column| Rectangle {
                    x: column * 8,
                    y: row * 8,
                    width: 8,
                    height: 8,
                })
        })
        .collect();
    surface
        .conn
        .poly_fill_rectangle(checker, gc, &squares)?
        .check()?;
    surface.conn.free_gc(gc)?.check()?;
    // One request drew the pattern, so one of its squares showing means all of them do.
    surface.until("checkered pattern", || Ok(surface.pixel((152, 12))? == 0))?;
    let mut windows = vec![checker];
    for (x, y, opacity) in [(20, 20, None), (170, 30, Some(0x8000_0000_u32))] {
        let window = create(
            surface,
            Rectangle {
                x,
                y,
                width: 100,
                height: 100,
            },
            RED,
        )?;
        surface
            .conn
            .change_property8(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_CLASS,
                AtomEnum::STRING,
                CLASS,
            )?
            .check()?;
        if let Some(opacity) = opacity {
            surface
                .conn
                .change_property32(
                    PropMode::REPLACE,
                    window,
                    surface.atom("_NET_WM_WINDOW_OPACITY")?,
                    AtomEnum::CARDINAL,
                    &[opacity],
                )?
                .check()?;
        }
        surface.conn.map_window(window)?.check()?;
        windows.push(window);
    }
    let red = |pixel: u32| (pixel >> 16) & 255;
    let green = |pixel: u32| (pixel >> 8) & 255;
    surface.until("rounded corner fixture", || {
        let translucent = surface.pixel((220, 80))?;
        Ok(surface.pixel((70, 70))? == RED
            && red(translucent) > 120
            && green(translucent) < 100
            && green(surface.pixel((20, 20))?) > 100)
    })?;
    // Beyond each arc the windows leave what lies beneath them uncolored.
    for point in [(20, 20), (119, 20), (20, 119), (170, 30), (269, 129)] {
        let pixel = surface.pixel(point)?;
        ensure!(
            red(pixel) == green(pixel) || green(pixel) > 100,
            "corner pixel {point:?} shows the window: {pixel:#08x}"
        );
    }
    snapshot(surface, args, "corners")?;
    for window in windows {
        surface.conn.destroy_window(window)?.check()?;
    }
    println!("PASS: rounded corners, their shadows, and blur at the arcs");
    Ok(())
}
