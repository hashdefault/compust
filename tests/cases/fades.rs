use crate::{
    rect,
    regions::{BACKGROUND, area},
    support::Desktop,
};
use anyhow::{Context as _, Result};
use std::time::Instant;
use x11rb::{connection::Connection as _, protocol::xproto::*};

#[test]
fn closes_window_destroyed_during_opening() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |[r, _, _]| (80..140).contains(&r))?;

    desktop.conn.destroy_window(window)?.check()?;

    desktop.until_pixel((40, 40), |[r, _, _]| (40..70).contains(&r))?;
    desktop.screenshot("fade-interrupted-close")?;
    desktop.until_pixel((40, 40), |p| p == [24, 24, 32])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn resumes_opacity_and_recaptures_content_when_remapped_during_close() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    desktop.conn.unmap_window(window)?.check()?;
    let started = Instant::now();
    let before = desktop.until_pixel((40, 40), |[r, _, _]| (160..200).contains(&r))?;
    let observed = Instant::now();

    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().background_pixel(0x0000_ff00),
        )?
        .check()?;
    desktop.map(window)?;

    let after = desktop.until_pixel((40, 40), |[_, g, _]| g > 24)?;
    // Allow one 120 Hz frame and rounding, plus the maximum smoothstep slope.
    let allowance = u8::try_from((6 + observed.elapsed().as_millis() * 383 / 1000).min(255))?;
    let [red, _, _] = before;
    let [_, green, _] = after;
    assert!(
        green >= red.saturating_sub(allowance),
        "remap restarted opacity: {before:?} -> {after:?}, elapsed {:?}, closing {:?}",
        observed.elapsed(),
        started.elapsed()
    );
    desktop.screenshot("fade-remapped-content")?;
    desktop.until_pixel((40, 40), |p| p == [0, 255, 0])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn preserves_stacking_of_destroyed_window_during_fade() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let upper = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.map(upper)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    let marker = desktop.window(rect(180, 20), 0x0000_ff00)?;
    desktop.screenshot("fade-destroyed-stacking-before")?;

    desktop.conn.grab_server()?.check()?;
    desktop.conn.destroy_window(lower)?.check()?;
    desktop.map(marker)?;
    desktop.conn.ungrab_server()?.check()?;

    desktop.until_pixel((200, 40), |[_, g, _]| (40..100).contains(&g))?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
    desktop.screenshot("fade-destroyed-stacking")?;
    desktop.until_pixel((200, 40), |p| p == [0, 255, 0])?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
    desktop.screenshot("fade-destroyed-stacking-settled")?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

/// Two tiled windows, the right one above the left in the stack, when `close` takes the left
/// one away and the right one grows over its place at once, as a tiling manager does, and then
/// draws itself at its new size: the closing window keeps fading over the neighbor instead of
/// vanishing under it.
fn closes_beside_a_neighbor(
    name: &str,
    close: impl FnOnce(&Desktop, Window) -> Result<()>,
) -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 1000\nblur_radius = 0")?;
    let left = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let right = desktop.window(rect(120, 20), 0x0000_00ff)?;
    desktop.map(left)?;
    desktop.map(right)?;
    desktop.until_pixel((60, 60), |p| p == [255, 0, 0])?;
    desktop.until_pixel((160, 60), |p| p == [0, 0, 255])?;

    desktop.conn.grab_server()?.check()?;
    close(&desktop, left)?;
    desktop
        .conn
        .configure_window(right, &ConfigureWindowAux::new().x(20).width(200))?
        .check()?;
    desktop.conn.ungrab_server()?.check()?;
    // Once the fade has begun, Compust watches what the neighbor draws.
    desktop.until_pixel((60, 60), |[r, _, _]| r < 255)?;
    desktop.fill(right, area(0, 0, 200, 100), 0x0000_00ff)?;

    let [red, _, blue] = desktop.until_pixel((60, 60), |[r, _, b]| r > 40 && b > 40)?;
    desktop.screenshot(name)?;
    assert!(
        red > 40 && blue > 40,
        "closing window under its neighbor: {red} {blue}"
    );
    desktop.until_pixel((60, 60), |p| p == [0, 0, 255])?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn destroyed_window_fades_over_the_neighbor_that_takes_its_place() -> Result<()> {
    closes_beside_a_neighbor("fade-destroyed-over-neighbor", |desktop, window| {
        desktop.conn.destroy_window(window)?.check()?;
        Ok(())
    })
}

#[test]
fn unmapped_window_fades_over_the_neighbor_that_takes_its_place() -> Result<()> {
    closes_beside_a_neighbor("fade-unmapped-over-neighbor", |desktop, window| {
        desktop.conn.unmap_window(window)?.check()?;
        Ok(())
    })
}

/// A window the test draws into to know that Compust has handled every event before.
struct Marker {
    window: Window,
    colors: std::iter::Cycle<std::array::IntoIter<u32, 2>>,
}

impl Marker {
    fn new(desktop: &Desktop) -> Result<Self> {
        let window = desktop.window(area(250, 150, 40, 40), 0x0080_8080)?;
        desktop.map(window)?;
        desktop.until_pixel((270, 170), |p| p == [128, 128, 128])?;
        Ok(Self {
            window,
            colors: [0x00ff_ff00, 0x0000_ffff].into_iter().cycle(),
        })
    }

    /// Wait until Compust shows a new color in the marker, which it draws after it has handled
    /// every earlier drawing.
    fn sync(&mut self, desktop: &Desktop) -> Result<()> {
        let color = self.colors.next().context("no marker color")?;
        desktop.fill(self.window, area(0, 0, 40, 40), color)?;
        let rgb = color.to_be_bytes();
        desktop.until_pixel((270, 170), |p| p[..] == rgb[1..])?;
        Ok(())
    }
}

/// Two tiled windows, as in `closes_beside_a_neighbor`, where the right one keeps no
/// background, as browsers' windows do, so that the server fills its new pixmap with the
/// screen when it grows over the left one's place. Its client draws nothing at first, and its
/// window manager repaints its border in another color, as Xmonad does when it gains focus.
/// When it `draws`, it draws one half and then the other.
fn grows_without_drawing(name: &str, border: u16, draws: bool) -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 100\nblur_radius = 0")?;
    let mut marker = Marker::new(&desktop)?;
    let left = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let right = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            24,
            right,
            desktop.root,
            120,
            20,
            96,
            96,
            border,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixmap(x11rb::NONE)
                .border_pixel(0x0080_8080),
        )?
        .check()?;
    desktop.map(left)?;
    desktop.map(right)?;
    desktop.fill(right, area(0, 0, 96, 96), 0x0000_00ff)?;
    desktop.until_pixel((60, 60), |p| p == [255, 0, 0])?;
    desktop.until_pixel((160, 60), |p| p == [0, 0, 255])?;

    // Under the grab, Compust starts watching the neighbor only after it has grown.
    desktop.conn.grab_server()?.check()?;
    desktop.conn.unmap_window(left)?.check()?;
    desktop
        .conn
        .configure_window(right, &ConfigureWindowAux::new().x(20).width(196))?
        .check()?;
    desktop.conn.ungrab_server()?.check()?;
    desktop
        .conn
        .change_window_attributes(
            right,
            &ChangeWindowAttributesAux::new().border_pixel(0x0000_ff00),
        )?
        .check()?;

    // The closing window fades out over what lay beneath it, and leaves no image behind.
    desktop.until_pixel((60, 60), |p| p == BACKGROUND)?;
    desktop.screenshot(name)?;
    assert_eq!(desktop.pixel((160, 60))?, [0, 0, 255]);
    if draws {
        // Until all of it is drawn, the neighbor keeps showing its previous capture, and once
        // it is, the new one shows at once, well before the hold would end by itself.
        desktop.fill(right, area(0, 0, 98, 96), 0x0000_00ff)?;
        marker.sync(&desktop)?;
        assert_eq!(desktop.pixel((60, 60))?, BACKGROUND);
        desktop.fill(right, area(98, 0, 98, 96), 0x0000_00ff)?;
        marker.sync(&desktop)?;
        assert_eq!(desktop.pixel((60, 60))?, [0, 0, 255]);
    } else {
        desktop.until_pixel((60, 60), |p| p != BACKGROUND)?;
    }
    if border > 0 {
        assert_eq!(desktop.pixel((21, 60))?, [0, 255, 0]);
    }
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn closing_window_leaves_no_image_in_the_neighbor_that_grows_over_it() -> Result<()> {
    grows_without_drawing("fade-no-image-in-neighbor", 2, true)
}

#[test]
fn closing_window_leaves_no_image_in_a_borderless_neighbor() -> Result<()> {
    grows_without_drawing("fade-no-image-in-borderless-neighbor", 0, true)
}

#[test]
fn neighbor_that_never_draws_shows_its_new_size_after_a_while() -> Result<()> {
    grows_without_drawing("fade-neighbor-never-draws", 2, false)
}

/// A window that draws into a child of another depth, as browsers do: the server copies the
/// child's pixels into the window. When the window grows over a closing one, the server fills
/// the new parts of both from what the screen showed, and copies them in as the browser
/// resizes the child before it draws: neither shows the closing window once it has faded.
#[test]
fn closing_window_leaves_no_image_in_a_neighbor_that_draws_into_a_child() -> Result<()> {
    let mut desktop = Desktop::new("fade_ms = 300\nblur_radius = 0")?;
    let mut marker = Marker::new(&desktop)?;
    let visual = desktop
        .conn
        .setup()
        .roots
        .iter()
        .flat_map(|screen| &screen.allowed_depths)
        .filter(|depth| depth.depth == 32)
        .flat_map(|depth| &depth.visuals)
        .find(|visual| visual.class == VisualClass::TRUE_COLOR)
        .map(|visual| visual.visual_id)
        .context("the server has no 32-bit visual")?;
    let colormap = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_colormap(ColormapAlloc::NONE, colormap, desktop.root, visual)?
        .check()?;
    let left = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            32,
            left,
            desktop.root,
            20,
            20,
            96,
            96,
            2,
            WindowClass::INPUT_OUTPUT,
            visual,
            &CreateWindowAux::new()
                .background_pixmap(x11rb::NONE)
                .border_pixel(0xff80_8080)
                .colormap(colormap),
        )?
        .check()?;
    let screen = desktop
        .conn
        .setup()
        .roots
        .first()
        .context("missing screen")?
        .clone();
    let child = desktop.conn.generate_id()?;
    desktop
        .conn
        .create_window(
            24,
            child,
            left,
            0,
            0,
            96,
            96,
            0,
            WindowClass::INPUT_OUTPUT,
            screen.root_visual,
            &CreateWindowAux::new()
                .background_pixmap(x11rb::NONE)
                .border_pixel(0)
                .colormap(screen.default_colormap),
        )?
        .check()?;
    let right = desktop.window(rect(120, 20), 0x00ff_0000)?;
    desktop.map(child)?;
    desktop.map(left)?;
    desktop.map(right)?;
    desktop.fill(child, area(0, 0, 96, 96), 0x0000_00ff)?;
    desktop.until_pixel((60, 60), |p| p == [0, 0, 255])?;
    desktop.until_pixel((170, 60), |p| p == [255, 0, 0])?;

    // The window manager grows the neighbor once the window is gone, and the browser follows
    // its window's size at once, drawing only later.
    desktop.conn.unmap_window(right)?.check()?;
    desktop.until_pixel((170, 60), |[r, _, _]| r < 255)?;
    desktop
        .conn
        .configure_window(left, &ConfigureWindowAux::new().width(196))?
        .check()?;
    desktop
        .conn
        .configure_window(child, &ConfigureWindowAux::new().width(196))?
        .check()?;

    desktop.until_pixel((170, 60), |p| p == BACKGROUND)?;
    desktop.screenshot("fade-no-image-in-browser-neighbor")?;
    assert_eq!(desktop.pixel((60, 60))?, [0, 0, 255]);
    desktop.fill(child, area(0, 0, 196, 96), 0x0000_ff00)?;
    marker.sync(&desktop)?;
    assert_eq!(desktop.pixel((170, 60))?, [0, 255, 0]);
    assert_eq!(desktop.pixel((60, 60))?, [0, 255, 0]);
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
