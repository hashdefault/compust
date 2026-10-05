use crate::{
    rect,
    regions::{area, assert_same, full_repaint, striped},
    rules::{atom, classed, set_types},
    support::Desktop,
};
use anyhow::{Context, Result, ensure};
use x11rb::{
    connection::Connection as _,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{
            AtomEnum, ClipOrdering, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux,
            PropMode, Rectangle, Window, WindowClass,
        },
    },
    wrapper::ConnectionExt as _,
};

const WHITE: [u8; 3] = [255, 255, 255];
const RED: [u8; 3] = [255, 0, 0];
const BLUE: [u8; 3] = [0, 0, 255];
/// Corners of radius 13, without fades or blur.
const ROUND: &str = "fade_ms = 0\nblur_radius = 0\ncorner_radius = 13\n";

/// White paper over the whole screen, typed as the desktop so that its own corners stay
/// square.
fn paper(desktop: &Desktop) -> Result<Window> {
    let paper = desktop.window(area(0, 0, 320, 240), 0x00ff_ffff)?;
    set_types(
        desktop,
        paper,
        &[atom(desktop, b"_NET_WM_WINDOW_TYPE_DESKTOP")?],
    )?;
    desktop.map(paper)?;
    desktop.until_pixel((10, 10), |p| p == WHITE)?;
    Ok(paper)
}

/// The color at `(x, y)` of a whole-screen image from `Desktop::image`.
fn at(image: &[u8], x: i32, y: i32) -> Result<[u8; 3]> {
    let index = usize::try_from(y * 320 + x)? * 4;
    match image.get(index..index + 3) {
        Some(&[blue, green, red]) => Ok([red, green, blue]),
        _ => anyhow::bail!("({x}, {y}) is outside the image"),
    }
}

/// How much of the pixel at `(column, row)` of the top-left corner square lies inside a
/// circle of `radius` centered on the square's inner corner, from 0 to 1, integrated
/// numerically across the pixel independently of the compositor's own table.
fn coverage(radius: i32, column: i32, row: i32) -> f64 {
    let r = f64::from(radius);
    let steps = 2000;
    let inside: f64 = (0..steps)
        .map(|step| {
            let x = f64::from(column) + (f64::from(step) + 0.5) / f64::from(steps);
            let half = (r * r - (x - r) * (x - r)).max(0.0).sqrt();
            let (low, high) = (
                f64::from(row).max(r - half),
                f64::from(row + 1).min(r + half),
            );
            (high - low).max(0.0)
        })
        .sum();
    inside / f64::from(steps)
}

/// How much of the pixel at `(x, y)` the window at `bounds`, with corners of `radius`, covers.
fn covered((left, top, width, height): (i32, i32, i32, i32), radius: i32, x: i32, y: i32) -> f64 {
    let (column, row) = (x - left, y - top);
    if !(0..width).contains(&column) || !(0..height).contains(&row) {
        return 0.0;
    }
    let (across, down) = (column.min(width - 1 - column), row.min(height - 1 - row));
    if across < radius && down < radius {
        coverage(radius, across, down)
    } else {
        1.0
    }
}

/// Check every pixel of the window at `bounds` and two pixels around it against `expected`,
/// which gives the color shown where the window covers a share of a pixel. Wholly covered
/// and uncovered pixels must match within rounding, and pixels the arcs cross within four
/// levels: the compositor counts 16 × 16 points per pixel where this integrates.
fn check(
    image: &[u8],
    bounds: (i32, i32, i32, i32),
    radius: i32,
    expected: impl Fn(f64) -> [f64; 3],
) -> Result<()> {
    check_at(image, bounds, radius, |_, _, share| expected(share))
}

/// `check`, with the expected color also depending on the pixel.
fn check_at(
    image: &[u8],
    bounds: (i32, i32, i32, i32),
    radius: i32,
    expected: impl Fn(i32, i32, f64) -> [f64; 3],
) -> Result<()> {
    let (left, top, width, height) = bounds;
    for y in top - 2..top + height + 2 {
        for x in left - 2..left + width + 2 {
            let share = covered(bounds, radius, x, y);
            let tolerance = if share > 0.0 && share < 1.0 { 4.0 } else { 1.0 };
            let shown = at(image, x, y)?;
            let wanted = expected(x, y, share);
            for (channel, (shown, wanted)) in shown.iter().zip(wanted).enumerate() {
                ensure!(
                    (f64::from(*shown) - wanted).abs() <= tolerance,
                    "({x}, {y}) covered {share:.3}: channel {channel} is {shown}, not {wanted:.1}"
                );
            }
        }
    }
    Ok(())
}

fn check_bordered(
    image: &[u8],
    bounds: (i32, i32, i32, i32),
    radius: i32,
    border: i32,
    opacity: f64,
) -> Result<()> {
    let (left, top, width, height) = bounds;
    for y in top - 2..top + height + 2 {
        for x in left - 2..left + width + 2 {
            let outer = covered(bounds, radius, x, y);
            let column = x - left;
            let row = y - top;
            let (content, ring) = if (0..width).contains(&column) && (0..height).contains(&row) {
                let (across, down) = (column.min(width - 1 - column), row.min(height - 1 - row));
                if across < radius && down < radius {
                    let inner = if across >= border && down >= border {
                        coverage(radius - border, across - border, down - border)
                    } else {
                        0.0
                    };
                    (inner, (outer - inner).max(0.0))
                } else if column < border
                    || column >= width - border
                    || row < border
                    || row >= height - border
                {
                    (0.0, 1.0)
                } else {
                    (1.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };
            let paper = 255.0 * (1.0 - opacity * outer);
            let wanted = [
                paper + 255.0 * opacity * content,
                paper,
                paper + 255.0 * opacity * ring,
            ];
            let shown = at(image, x, y)?;
            for (channel, (shown, wanted)) in shown.iter().zip(wanted).enumerate() {
                ensure!(
                    (f64::from(*shown) - wanted).abs() <= 5.0,
                    "({x}, {y}) content {content:.3}, border {ring:.3}: channel {channel} is {shown}, not {wanted:.1}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn corners_follow_the_arc_over_opaque_translucent_and_argb_windows() -> Result<()> {
    let desktop = Desktop::new(ROUND)?;
    paper(&desktop)?;
    let opaque = desktop.window(rect(5, 20), 0x00ff_0000)?;
    let translucent = desktop.window(rect(110, 20), 0x00ff_0000)?;
    desktop.opacity(translucent, 0x8000_0000)?;
    let argb = desktop.argb_window(rect(215, 20))?;
    for window in [opaque, translucent, argb] {
        desktop.map(window)?;
    }
    // Half-transparent red, premultiplied.
    desktop.fill(argb, area(0, 0, 100, 100), 0x8080_0000)?;
    desktop.until_pixel((265, 70), |[r, g, _]| r == 255 && g < 200)?;
    desktop.until_pixel((5, 20), |p| p == WHITE)?;
    let image = desktop.image()?;
    desktop.screenshot("corners")?;
    let paper = 255.0;
    check(&image, (5, 20, 100, 100), 13, |share| {
        [paper, paper * (1.0 - share), paper * (1.0 - share)]
    })?;
    let half = f64::from(0x8000_u16) / f64::from(u16::MAX);
    check(&image, (110, 20, 100, 100), 13, |share| {
        let alpha = share * half;
        [paper, paper * (1.0 - alpha), paper * (1.0 - alpha)]
    })?;
    let alpha = 128.0 / 255.0;
    check(&image, (215, 20, 100, 100), 13, |share| {
        let shown = paper * (1.0 - alpha * share);
        [128.0 * share + shown, shown, shown]
    })?;
    Ok(())
}

#[test]
fn borders_round_with_their_window() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\ncorner_radius = 15\n")?;
    paper(&desktop)?;
    let bordered = |x, opacity: Option<u32>| -> Result<Window> {
        let window = desktop.conn.generate_id()?;
        desktop
            .conn
            .create_window(
                24,
                window,
                desktop.root,
                x,
                60,
                100,
                100,
                2,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new()
                    .background_pixel(0x00ff_0000)
                    .border_pixel(0x0000_00ff),
            )?
            .check()?;
        if let Some(opacity) = opacity {
            desktop.opacity(window, opacity)?;
        }
        desktop.map(window)?;
        Ok(window)
    };
    bordered(20, None)?;
    bordered(160, Some(0x8000_0000))?;
    desktop.until_pixel((70, 110), |p| p == RED)?;
    desktop.until_pixel((20, 60), |p| p == WHITE)?;
    desktop.until_pixel((210, 110), |[r, g, b]| {
        r == 255 && (126..=128).contains(&g) && (126..=128).contains(&b)
    })?;
    let image = desktop.image()?;
    check_bordered(&image, (20, 60, 104, 104), 15, 2, 1.0)?;
    let half = f64::from(0x8000_u16) / f64::from(u16::MAX);
    check_bordered(&image, (160, 60, 104, 104), 15, 2, half)?;
    Ok(())
}

#[test]
fn blur_stops_at_the_arcs() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 4\ncorner_radius = 13\n")?;
    striped(&desktop, 320)?;
    // The stripes are drawn one by one; wait for the last ones before taking the scene.
    desktop.until_pixel((318, 200), |p| p == [0, 0, 0])?;
    desktop.until_pixel((319, 200), |p| p == WHITE)?;
    desktop.wait_vblanks(2)?;
    let before = desktop.image()?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.opacity(window, 0x8000_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((150, 110), |[r, _, _]| r > 100 && r < 255)?;
    let image = desktop.image()?;
    let mut outside = 0;
    for y in 60..160 {
        for x in 100..200 {
            let share = covered((100, 60, 100, 100), 13, x, y);
            if share == 0.0 {
                outside += 1;
                assert_eq!(
                    at(&image, x, y)?,
                    at(&before, x, y)?,
                    "({x}, {y}) beyond the arc shows blur or the window"
                );
            }
        }
    }
    ensure!(outside > 100, "only {outside} pixels lie beyond the arcs");
    // Away from its corners, the window and its blur are the same as with square corners.
    desktop.reload("fade_ms = 0\nblur_radius = 4\ncorner_radius = 0\n")?;
    desktop.until_pixel((100, 60), |p| p != at(&before, 100, 60).unwrap_or_default())?;
    desktop.wait_vblanks(2)?;
    let square = desktop.image()?;
    let mut same = 0;
    for y in 60..160 {
        for x in 100..200 {
            let corner = !(113..187).contains(&x) && !(73..147).contains(&y);
            if !corner {
                same += 1;
                assert_eq!(
                    at(&image, x, y)?,
                    at(&square, x, y)?,
                    "({x}, {y}) differs from square corners"
                );
            }
        }
    }
    ensure!(same > 9000, "only {same} pixels compared");
    ensure!(
        at(&square, 150, 110)? != at(&before, 150, 110)?,
        "the window does not show"
    );
    Ok(())
}

#[test]
fn windows_beneath_show_through_rounded_corners_of_opaque_ones() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4\ncorner_radius = 13\n";
    let desktop = Desktop::new(settings)?;
    paper(&desktop)?;
    let beneath = desktop.window(area(80, 40, 140, 140), 0x0000_00ff)?;
    desktop.map(beneath)?;
    let above = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.map(above)?;
    desktop.until_pixel((150, 110), |p| p == RED)?;
    desktop.until_pixel((100, 60), |p| p == BLUE)?;
    // Repainting the window beneath reaches the corners of the opaque one above it.
    desktop.fill(beneath, area(0, 0, 140, 140), 0x0000_ff00)?;
    desktop.until_pixel((100, 60), |p| p == [0, 255, 0])?;
    let image = desktop.image()?;
    check(&image, (100, 60, 100, 100), 13, |share| {
        [255.0 * share, 255.0 * (1.0 - share), 0.0]
    })?;
    assert_same(
        &image,
        &full_repaint(&desktop, settings)?,
        "a rounded window over another",
    );
    Ok(())
}

/// A red window at `rect`, 60 pixels square, with a notch in the middle of its bottom edge
/// that leaves its corners inside its shape.
fn notched(desktop: &Desktop, rect: Rectangle) -> Result<Window> {
    let window = desktop.window(rect, 0x00ff_0000)?;
    let shape = [
        Rectangle {
            x: 0,
            y: 0,
            width: 60,
            height: 50,
        },
        Rectangle {
            x: 0,
            y: 50,
            width: 20,
            height: 10,
        },
        Rectangle {
            x: 40,
            y: 50,
            width: 20,
            height: 10,
        },
    ];
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
    Ok(window)
}

#[test]
fn rules_choose_radii_and_some_windows_stay_square() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 0\ncorner_radius = 10\n\n\
         [[rules]]\nwm_class = \"Square\"\ncorner_radius = 0\n\n\
         [[rules]]\nwindow_type = \"dock\"\ncorner_radius = 6\n";
    let desktop = Desktop::new(settings)?;
    paper(&desktop)?;
    let place = |x: i16, y: i16| Rectangle {
        x,
        y,
        width: 60,
        height: 60,
    };
    let typed = |rect, kind: &[u8]| -> Result<Window> {
        let window = desktop.window(rect, 0x00ff_0000)?;
        set_types(&desktop, window, &[atom(&desktop, kind)?])?;
        Ok(window)
    };
    let normal = desktop.window(place(10, 10), 0x00ff_0000)?;
    let square = classed(&desktop, place(80, 10), "Square")?;
    let menu = typed(place(150, 10), b"_NET_WM_WINDOW_TYPE_MENU")?;
    let dock = typed(place(220, 10), b"_NET_WM_WINDOW_TYPE_DOCK")?;
    let fullscreen = desktop.window(place(10, 90), 0x00ff_0000)?;
    let state = atom(&desktop, b"_NET_WM_STATE")?;
    let full = atom(&desktop, b"_NET_WM_STATE_FULLSCREEN")?;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            fullscreen,
            state,
            AtomEnum::ATOM,
            &[full],
        )?
        .check()?;
    let decorated = desktop.window(place(80, 90), 0x00ff_0000)?;
    let extents = atom(&desktop, b"_GTK_FRAME_EXTENTS")?;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            decorated,
            extents,
            AtomEnum::CARDINAL,
            &[4, 4, 4, 4],
        )?
        .check()?;
    let shaped = notched(&desktop, place(150, 90))?;
    let windows = [
        (normal, (10, 10), 10),
        (square, (80, 10), 0),
        (menu, (150, 10), 0),
        (dock, (220, 10), 6),
        (fullscreen, (10, 90), 0),
        (decorated, (80, 90), 0),
        (shaped, (150, 90), 0),
    ];
    for (window, _, _) in windows {
        desktop.map(window)?;
    }
    desktop.until_pixel((155, 95), |p| p == RED)?;
    desktop.until_pixel((10, 10), |p| p == WHITE)?;
    let image = desktop.image()?;
    // The notch of the shaped window shows the paper.
    let notch = |x: i32, y: i32| (170..190).contains(&x) && (140..150).contains(&y);
    for (window, (x, y), radius) in windows {
        check_at(&image, (x, y, 60, 60), radius, |x, y, share| {
            let shown = if notch(x, y) { 0.0 } else { share };
            [255.0, 255.0 * (1.0 - shown), 255.0 * (1.0 - shown)]
        })
        .with_context(|| format!("window {window:#x} at ({x}, {y})"))?;
    }
    // A reload that rounds nothing makes the normal window square again.
    desktop.reload(&settings.replace("corner_radius = 10", "corner_radius = 0"))?;
    desktop.until_pixel((10, 10), |p| p == RED)?;
    Ok(())
}

#[test]
fn rounded_windows_over_the_screen_stay_composited_until_fullscreen() -> Result<()> {
    let desktop = Desktop::new(
        "fade_ms = 0\nblur_radius = 0\ncorner_radius = 10\nunredirect_fullscreen = true\n",
    )?;
    let cover = desktop.window(area(0, 0, 320, 240), 0x0000_00ff)?;
    desktop.map(cover)?;
    desktop.until_pixel((160, 120), |p| p == BLUE)?;
    // Its rounded corners show the background, so it hides nothing there.
    desktop.until_pixel((0, 0), |p| p != BLUE)?;
    desktop.wait_vblanks(4)?;
    ensure!(
        !desktop.suspended()?,
        "compositing stopped behind rounded corners"
    );
    // Fullscreen, it is square and covers everything.
    let state = atom(&desktop, b"_NET_WM_STATE")?;
    let full = atom(&desktop, b"_NET_WM_STATE_FULLSCREEN")?;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, cover, state, AtomEnum::ATOM, &[full])?
        .check()?;
    desktop.until_suspended(true)?;
    desktop.conn.delete_property(cover, state)?.check()?;
    desktop.until_suspended(false)?;
    desktop.until_pixel((0, 0), |p| p != BLUE)?;
    Ok(())
}

#[test]
fn changing_radii_leaks_nothing() -> Result<()> {
    let desktop = Desktop::new(ROUND)?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.opacity(window, 0x8000_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((100, 60), |p| p == WHITE)?;
    let cycle = |radius: u8| -> Result<()> {
        desktop.reload(&ROUND.replace("13", &radius.to_string()))?;
        // The pixel one in from the corner is outside the arc from radius 8.
        desktop.until_pixel((101, 61), |p| p == WHITE)?;
        desktop.wait_vblanks(2)
    };
    // Each radius needs a disk of its own size; one that no window uses any more must go.
    cycle(8)?;
    let baseline = desktop.resources()?;
    for radius in [9, 10, 11, 12] {
        cycle(radius)?;
        assert_eq!(
            desktop.resources()?.counts(),
            baseline.counts(),
            "radius {radius}"
        );
    }
    cycle(8)?;
    assert_eq!(desktop.resources()?, baseline);
    Ok(())
}

/// The weights of three box filters in a row reaching `radius` pixels each way, as the
/// documentation describes shadows: their widths add up to `2 × radius + 3`.
fn box_kernel(radius: i32) -> Vec<f64> {
    let span = 2 * radius + 3;
    let (each, rest) = (span / 3, span % 3);
    let widths = [each + i32::from(rest > 0), each + i32::from(rest > 1), each];
    let mut kernel = vec![1.0];
    for width in widths {
        let width = usize::try_from(width).unwrap_or(1);
        let mut spread = vec![0.0; kernel.len() + width - 1];
        for (index, weight) in kernel.iter().enumerate() {
            for slot in spread.iter_mut().skip(index).take(width) {
                *slot += weight;
            }
        }
        kernel = spread;
    }
    kernel
}

#[test]
fn shadows_follow_rounded_corners_and_fill_their_gaps() -> Result<()> {
    let desktop = Desktop::new(
        "fade_ms = 0\nblur_radius = 0\ncorner_radius = 13\n\
         shadow_radius = 12\nshadow_opacity = 100\nshadow_offset_x = 4\nshadow_offset_y = 6\n",
    )?;
    paper(&desktop)?;
    desktop.map(desktop.window(rect(100, 60), 0x00ff_0000)?)?;
    desktop.until_pixel((150, 110), |p| p == RED)?;
    desktop.until_pixel((203, 110), |[r, _, _]| r < 255)?;
    let image = desktop.image()?;
    desktop.screenshot("rounded-shadow")?;
    // The shadow blurs the rounded window moved by the offset, (104, 66), over 12 pixels.
    let kernel = box_kernel(12);
    let total: f64 = kernel.iter().sum();
    let window = (100, 60, 100, 100);
    // The window's coverage, integrated once for its 100 × 100 pixels.
    let coverage: Vec<f64> = (0..100)
        .flat_map(|row| (0..100).map(move |column| covered((0, 0, 100, 100), 13, column, row)))
        .collect();
    let moved = |x: i32, y: i32| -> f64 {
        // The rounded window moved by the offset, to (104, 66).
        let (column, row) = (x - 104, y - 66);
        if !(0..100).contains(&column) || !(0..100).contains(&row) {
            return 0.0;
        }
        usize::try_from(row * 100 + column)
            .ok()
            .and_then(|at| coverage.get(at).copied())
            .unwrap_or(0.0)
    };
    for y in 50..186 {
        for x in 86..222 {
            let mut dark = 0.0;
            for (k, across) in kernel.iter().enumerate() {
                for (l, down) in kernel.iter().enumerate() {
                    let (dx, dy) = (i32::try_from(k)? - 12, i32::try_from(l)? - 12);
                    dark += across * down * moved(x + dx, y + dy);
                }
            }
            dark /= total * total;
            let share = covered(window, 13, x, y);
            if share >= 1.0 {
                continue;
            }
            // The shadow fills a corner gap as much as the window leaves it uncovered.
            let beneath = (100..200).contains(&x) && (60..160).contains(&y);
            let darkness = if beneath { dark * (1.0 - share) } else { dark };
            let paper = 255.0 * (1.0 - darkness) * (1.0 - share);
            let wanted = [255.0 * share + paper, paper, paper];
            let shown = at(&image, x, y)?;
            for (channel, (shown, wanted)) in shown.iter().zip(wanted).enumerate() {
                ensure!(
                    (f64::from(*shown) - wanted).abs() <= 4.0,
                    "({x}, {y}): channel {channel} is {shown}, not {wanted:.1}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn rounded_shadows_stay_out_of_translucent_windows_and_match_a_full_repaint() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4\ncorner_radius = 13\n\
                    shadow_radius = 12\nshadow_opacity = 100\n";
    let desktop = Desktop::new(settings)?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.opacity(window, 0x8000_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((203, 110), |[r, _, _]| r < 255)?;
    // Half red over white inside, as bright as without a shadow.
    for point in [(150, 110), (110, 70), (189, 149)] {
        let [r, g, b] = desktop.pixel(point)?;
        ensure!(
            r == 255 && (126..=129).contains(&g) && g == b,
            "{point:?}: {r} {g} {b}"
        );
    }
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().x(130).y(80))?
        .check()?;
    desktop.until_pixel((235, 130), |[r, _, _]| r == 255)?;
    desktop.wait_vblanks(2)?;
    let image = desktop.image()?;
    assert_same(
        &image,
        &full_repaint(&desktop, settings)?,
        "a moved rounded window with a shadow",
    );
    Ok(())
}
