use crate::{
    rect,
    regions::{area, assert_same, confined, full_repaint, striped},
    rules::{atom, classed, set_types},
    support::{Desktop, presentation::Presentation, requests::Requests},
};
use anyhow::{Context, Result};
use x11rb::{
    NONE,
    connection::Connection as _,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{
            AtomEnum, CREATE_PIXMAP_REQUEST, ClipOrdering, ConfigureWindowAux, ConnectionExt as _,
            CreateWindowAux, PUT_IMAGE_REQUEST, PropMode, StackMode, Window, WindowClass,
        },
    },
    wrapper::ConnectionExt as _,
};

const RED: [u8; 3] = [255, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
/// Shadows at full darkness, 12 pixels wide, without fades or blur.
const DARK: &str = "fade_ms = 0\nblur_radius = 0\nshadow_radius = 12\nshadow_opacity = 100\n";

/// A white window over the whole screen, on which shadows show as shades of gray.
fn paper(desktop: &Desktop) -> Result<Window> {
    let paper = desktop.window(area(0, 0, 320, 240), 0x00ff_ffff)?;
    desktop.map(paper)?;
    desktop.until_pixel((10, 10), |p| p == WHITE)?;
    Ok(paper)
}

/// The gray level at `point`, which must show no color.
fn gray(desktop: &Desktop, point: (i16, i16)) -> Result<u8> {
    let [r, g, b] = desktop.pixel(point)?;
    assert!(r == g && g == b, "{point:?} shows color: {r} {g} {b}");
    Ok(r)
}

#[test]
fn shadows_spread_by_the_radius_and_follow_the_offset() -> Result<()> {
    let settings = format!("{DARK}shadow_offset_x = 4\nshadow_offset_y = 6\n");
    let desktop = Desktop::new(&settings)?;
    paper(&desktop)?;
    desktop.map(desktop.window(rect(100, 60), 0x00ff_0000)?)?;
    desktop.until_pixel((150, 110), |p| p == RED)?;
    desktop.until_pixel((203, 110), |[r, _, _]| r < 255)?;
    desktop.screenshot("shadow")?;
    // The shadow is the window's rectangle, moved by the offset to (104, 66) and blurred over
    // 12 pixels: it covers (92, 54) to (216, 178). Along a row or column through the middle,
    // the darkness is the blur's profile, half at the moved edge.
    for (point, expected) in [
        // Right of the window: three pixels inside the moved edge, then outside it.
        ((200, 110), 58),
        ((203, 110), 117),
        ((204, 110), 138),
        ((210, 110), 235),
        ((215, 110), 255),
        ((216, 110), 255),
        // Left of it, the offset leaves 8 pixels of shadow.
        ((99, 110), 213),
        ((95, 110), 248),
        ((91, 110), 255),
        // Above, 6 pixels; below, 18.
        ((150, 59), 235),
        ((150, 53), 255),
        ((150, 160), 29),
        ((150, 170), 213),
        ((150, 178), 255),
    ] {
        let level = gray(&desktop, point)?;
        assert!(
            level.abs_diff(expected) <= 1,
            "{point:?}: {level}, not {expected}"
        );
    }
    // In a corner both profiles weigh in: 96/255 across times 138/255 down.
    let corner = gray(&desktop, (205, 165))?;
    assert!(corner.abs_diff(203) <= 2, "{corner}");
    // Darkness never grows with distance, and the window itself is untouched.
    let row = (200..217)
        .map(|x| gray(&desktop, (x, 110)))
        .collect::<Result<Vec<_>>>()?;
    assert!(row.is_sorted(), "{row:?}");
    for point in [(100, 60), (199, 159), (150, 110)] {
        assert_eq!(desktop.pixel(point)?, RED, "at {point:?}");
    }
    Ok(())
}

#[test]
fn shadows_keep_their_place_past_the_screen_edges() -> Result<()> {
    let desktop = Desktop::new(DARK)?;
    paper(&desktop)?;
    // Given a window whose shadow starts off the screen, the part in sight is the same as
    // anywhere else: two pixels outside an edge the full shadow leaves 179.
    let window = desktop.window(rect(5, 5), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((107, 55), |[r, _, _]| r == 179)?;
    assert_eq!(gray(&desktop, (55, 107))?, 179);
    assert_eq!(gray(&desktop, (117, 55))?, 255);
    // Half off the screen to the left and top, it still follows its right and bottom edges.
    let aux = ConfigureWindowAux::new().x(-50).y(-40);
    desktop.conn.configure_window(window, &aux)?.check()?;
    desktop.until_pixel((52, 20), |[r, _, _]| r == 179)?;
    assert_eq!(gray(&desktop, (20, 62))?, 179);
    assert_eq!(gray(&desktop, (107, 55))?, 255);
    Ok(())
}

#[test]
fn a_shadow_lies_around_its_window_never_beneath_it() -> Result<()> {
    let desktop = Desktop::new(DARK)?;
    paper(&desktop)?;
    // Given a half-transparent red window, the paper shows through it as bright as without
    // shadows: half red over white. A shadow beneath the window would darken it to a quarter.
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.opacity(window, 0x8000_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((202, 110), |[r, _, _]| r < 255)?;
    for point in [(150, 110), (100, 60), (199, 159)] {
        let [r, g, b] = desktop.pixel(point)?;
        assert!(
            r == 255 && (126..=129).contains(&g) && g == b,
            "{point:?}: {r} {g} {b}"
        );
    }
    Ok(())
}

/// Map a red window prepared by `prepare`, report whether a shadow shows beside it, and
/// remove it again.
fn casts(
    desktop: &Desktop,
    window: Window,
    prepare: &dyn Fn(Window) -> Result<()>,
) -> Result<bool> {
    prepare(window)?;
    desktop.map(window)?;
    desktop.until_pixel((120, 110), |p| p == RED)?;
    desktop.wait_vblanks(2)?;
    let beside = gray(desktop, (202, 110))?;
    desktop.conn.destroy_window(window)?.check()?;
    desktop.until_pixel((120, 110), |p| p == WHITE)?;
    desktop.until_pixel((202, 110), |p| p == WHITE)?;
    Ok(beside < 255)
}

fn set_extents(desktop: &Desktop, window: Window, margins: [u32; 4]) -> Result<()> {
    let property = atom(desktop, b"_GTK_FRAME_EXTENTS")?;
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            window,
            property,
            AtomEnum::CARDINAL,
            &margins,
        )?
        .check()?;
    Ok(())
}

fn shape_left_half(desktop: &Desktop, window: Window) -> Result<()> {
    desktop
        .conn
        .shape_rectangles(
            SO::SET,
            SK::BOUNDING,
            ClipOrdering::UNSORTED,
            window,
            0,
            0,
            &[area(0, 0, 50, 100)],
        )?
        .check()?;
    Ok(())
}

#[test]
fn shadows_follow_types_decorations_shapes_and_rules() -> Result<()> {
    let settings = format!(
        "{DARK}[[rules]]\nwm_class = \"Flat\"\nshadow = false\n\n[[rules]]\nwm_class = \"Bar\"\nshadow = true\n"
    );
    let mut owner = Desktop::new(&settings)?;
    let desktop = &owner;
    paper(desktop)?;
    let plain = || desktop.window(rect(100, 60), 0x00ff_0000);
    let typed =
        |name: &'static [u8]| move |window| set_types(desktop, window, &[atom(desktop, name)?]);
    assert!(casts(desktop, plain()?, &|_| Ok(()))?, "a plain window");
    for (name, expected) in [
        (&b"_NET_WM_WINDOW_TYPE_DIALOG"[..], true),
        (b"_NET_WM_WINDOW_TYPE_UTILITY", true),
        (b"_NET_WM_WINDOW_TYPE_DOCK", false),
        (b"_NET_WM_WINDOW_TYPE_DESKTOP", false),
        (b"_NET_WM_WINDOW_TYPE_MENU", false),
        (b"_NET_WM_WINDOW_TYPE_TOOLTIP", false),
        (b"_NET_WM_WINDOW_TYPE_NOTIFICATION", false),
    ] {
        let label = String::from_utf8_lossy(name);
        assert_eq!(casts(desktop, plain()?, &typed(name))?, expected, "{label}");
    }
    // A window the window manager leaves alone casts a shadow only when it names a type that
    // does: bars, menus, and tooltips of older toolkits name none.
    let unmanaged = || -> Result<Window> {
        let window = desktop.conn.generate_id()?;
        let aux = CreateWindowAux::new()
            .background_pixel(0x00ff_0000)
            .override_redirect(1);
        desktop
            .conn
            .create_window(
                24,
                window,
                desktop.root,
                100,
                60,
                100,
                100,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &aux,
            )?
            .check()?;
        Ok(window)
    };
    assert!(!casts(desktop, unmanaged()?, &|_| Ok(()))?, "unmanaged");
    assert!(
        casts(desktop, unmanaged()?, &typed(b"_NET_WM_WINDOW_TYPE_NORMAL"))?,
        "unmanaged but typed"
    );
    // A client that keeps margins for its own shadow gets none; empty margins hold none.
    let margins = |margins| move |window| set_extents(desktop, window, margins);
    assert!(
        !casts(desktop, plain()?, &margins([26, 26, 23, 29]))?,
        "margins"
    );
    assert!(casts(desktop, plain()?, &margins([0; 4]))?, "empty margins");
    // A shaped window would get the shadow of its bounding rectangle, so it gets none.
    let shaped = |window| shape_left_half(desktop, window);
    assert!(!casts(desktop, plain()?, &shaped)?, "shaped");
    // Rules decide before any of that.
    let flat = classed(desktop, rect(100, 60), "Flat")?;
    assert!(!casts(desktop, flat, &|_| Ok(()))?, "a rule against");
    let bar = classed(desktop, rect(100, 60), "Bar")?;
    assert!(
        casts(desktop, bar, &typed(b"_NET_WM_WINDOW_TYPE_DOCK"))?,
        "a rule for a dock"
    );
    assert!(owner.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn shadows_come_and_go_as_windows_and_settings_change() -> Result<()> {
    let mut desktop = Desktop::new(DARK)?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.map(window)?;
    let beside = (202, 110);
    let dark = |[r, _, _]: [u8; 3]| r < 200;
    desktop.until_pixel(beside, dark)?;
    // Margins set and removed while the window is open.
    set_extents(&desktop, window, [10; 4])?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    let extents = atom(&desktop, b"_GTK_FRAME_EXTENTS")?;
    desktop.conn.delete_property(window, extents)?.check()?;
    desktop.until_pixel(beside, dark)?;
    // A type that casts none, then one that does.
    let named = |name: &[u8]| set_types(&desktop, window, &[atom(&desktop, name)?]);
    named(b"_NET_WM_WINDOW_TYPE_DOCK")?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    named(b"_NET_WM_WINDOW_TYPE_DIALOG")?;
    desktop.until_pixel(beside, dark)?;
    // A shape, then the whole window again: the shadow leaves with the shape and returns.
    shape_left_half(&desktop, window)?;
    desktop.until_pixel((170, 110), |p| p == WHITE)?;
    assert_eq!(desktop.pixel(beside)?, WHITE);
    assert_eq!(desktop.pixel((152, 110))?, WHITE);
    desktop
        .conn
        .shape_mask(SO::SET, SK::BOUNDING, window, 0, 0, NONE)?
        .check()?;
    desktop.until_pixel(beside, dark)?;
    // Reloads turn shadows off and on, lighten them, and move them.
    desktop.reload("fade_ms = 0\nblur_radius = 0\n")?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    desktop.reload("fade_ms = 0\nblur_radius = 0\nshadow_radius = 12\nshadow_opacity = 50\n")?;
    // Two pixels outside the edge the profile is 76 of 255; at half darkness that leaves 217.
    desktop.until_pixel(beside, |[r, _, _]| r.abs_diff(217) <= 1)?;
    desktop.reload(&format!("{DARK}shadow_offset_x = -20\n"))?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    desktop.until_pixel((77, 110), |[r, _, _]| r.abs_diff(179) <= 1)?;
    // An opacity rule lightens the shadow with its window.
    desktop.reload(&format!(
        "{DARK}[[rules]]\nwindow_type = \"dialog\"\nopacity = 50\n"
    ))?;
    desktop.until_pixel(beside, |[r, _, _]| r.abs_diff(217) <= 1)?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn shadows_fade_with_their_windows() -> Result<()> {
    let desktop = Desktop::new(&DARK.replace("fade_ms = 0", "fade_ms = 400"))?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.map(window)?;
    // Two pixels outside the edge, the full shadow leaves 179; a fade passes through the
    // levels between and ends exactly there.
    let beside = (202, 110);
    let midway = |[r, _, _]: [u8; 3]| (190..250).contains(&r);
    desktop.until_pixel(beside, midway)?;
    desktop.until_pixel(beside, |[r, _, _]| r == 179)?;
    desktop.conn.destroy_window(window)?.check()?;
    desktop.until_pixel(beside, midway)?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    desktop.until_pixel((150, 110), |p| p == WHITE)?;
    Ok(())
}

#[test]
fn shadowed_scenes_match_a_full_repaint_with_present() -> Result<()> {
    shadowed_scenes_match_a_full_repaint(true)
}

#[test]
fn shadowed_scenes_match_a_full_repaint_with_xrender() -> Result<()> {
    shadowed_scenes_match_a_full_repaint(false)
}

/// Shadows change with their windows and lie in the scene that windows above them blur.
/// After every kind of change, the frame must match one painted whole by a new renderer.
fn shadowed_scenes_match_a_full_repaint(vsync: bool) -> Result<()> {
    let base = format!("fade_ms = 0\nblur_radius = 4\nvsync = {vsync}\n");
    let shadows = "shadow_radius = 12\nshadow_offset_y = 4\nshadow_opacity = 60\n";
    let settings = format!("{base}{shadows}");
    let (mut desktop, mut presentation) = Desktop::with_display(&settings, Presentation::start)?;
    let background = striped(&desktop, 320)?;
    let low = desktop.window(area(40, 40, 90, 70), 0x0000_ff00)?;
    let glass = desktop.window(rect(100, 60), 0x0000_0080)?;
    desktop.opacity(glass, 0x8000_0000)?;
    let top = desktop.window(area(180, 140, 60, 50), 0x0000_00ff)?;
    for window in [low, glass, top] {
        desktop.map(window)?;
    }
    desktop.until_pixel((230, 180), |p| p == [0, 0, 255])?;
    desktop.wait_vblanks(2)?;
    let step = |case: &str,
                settings: &str,
                within: Option<(i16, i16, u16, u16)>,
                change: &dyn Fn() -> Result<()>| {
        let before = presentation.updates()?.len();
        change()?;
        desktop.wait_vblanks(4)?;
        let shown = desktop.image()?;
        let updates = presentation.updates()?;
        let since = updates.get(before..).context("missing updates")?;
        if let Some(bounds) = within {
            assert!(confined(since, &[bounds]), "{case} repainted {since:?}");
        }
        assert_same(&shown, &full_repaint(&desktop, settings)?, case);
        anyhow::Ok(())
    };
    let configure = |window, aux: &ConfigureWindowAux| -> Result<()> {
        desktop.conn.configure_window(window, aux)?.check()?;
        Ok(())
    };

    step("the first frame", &settings, None, &|| Ok(()))?;
    // A window's own content leaves its shadow alone.
    step(
        "content of a shadowed window",
        &settings,
        Some((50, 50, 4, 4)),
        &|| desktop.fill(low, area(10, 10, 4, 4), 0x00ff_0000),
    )?;
    // A fade or opacity change repaints the window with its shadow, and no more.
    step(
        "opacity of a shadowed window",
        &settings,
        Some((88, 52, 124, 124)),
        &|| desktop.opacity(glass, 0xc000_0000),
    )?;
    step("a move beneath a blur", &settings, None, &|| {
        configure(low, &ConfigureWindowAux::new().x(20))
    })?;
    step("a resize", &settings, None, &|| {
        configure(low, &ConfigureWindowAux::new().width(70).height(90))
    })?;
    step("a move of the blurred window", &settings, None, &|| {
        configure(glass, &ConfigureWindowAux::new().x(120).y(50))
    })?;
    step("the lowest window raised", &settings, None, &|| {
        configure(low, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE))
    })?;
    step(
        "the top window lowered beneath the blur",
        &settings,
        None,
        &|| {
            let aux = ConfigureWindowAux::new()
                .sibling(glass)
                .stack_mode(StackMode::BELOW);
            configure(top, &aux)
        },
    )?;
    step("a window unmapped", &settings, None, &|| {
        desktop.conn.unmap_window(top)?.check()?;
        Ok(())
    })?;
    step("a window mapped", &settings, None, &|| desktop.map(top))?;
    step("the scene beneath a shadow", &settings, None, &|| {
        desktop.fill(background, area(10, 120, 60, 30), 0x00ff_0000)
    })?;
    step("a window that stops casting one", &settings, None, &|| {
        set_types(
            &desktop,
            low,
            &[atom(&desktop, b"_NET_WM_WINDOW_TYPE_DOCK")?],
        )
    })?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    presentation.finish()
}

#[test]
fn reloaded_shadow_settings_redraw_every_shadow() -> Result<()> {
    let base = "fade_ms = 0\nblur_radius = 4\n";
    let mut desktop = Desktop::new(&format!("{base}shadow_radius = 12\n"))?;
    striped(&desktop, 320)?;
    let low = desktop.window(area(40, 40, 90, 70), 0x0000_ff00)?;
    let glass = desktop.window(rect(100, 60), 0x0000_0080)?;
    desktop.opacity(glass, 0x8000_0000)?;
    for window in [low, glass] {
        desktop.map(window)?;
    }
    desktop.until_pixel((60, 60), |p| p == [0, 255, 0])?;
    for (case, shadows) in [
        (
            "lighter shadows",
            "shadow_radius = 12\nshadow_offset_y = 4\nshadow_opacity = 30\n",
        ),
        (
            "smaller shadows",
            "shadow_radius = 5\nshadow_offset_x = -3\nshadow_opacity = 30\n",
        ),
        ("no shadows", ""),
        (
            "wide shadows",
            "shadow_radius = 40\nshadow_offset_x = 9\nshadow_offset_y = -9\n",
        ),
    ] {
        let settings = format!("{base}{shadows}");
        desktop.reload(&settings)?;
        desktop.wait_vblanks(4)?;
        let shown = desktop.image()?;
        assert_same(&shown, &full_repaint(&desktop, &settings)?, case);
    }
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn shadows_show_around_what_hides_their_windows() -> Result<()> {
    let settings = "fade_ms = 0\nblur_radius = 4\nshadow_radius = 12\nshadow_opacity = 50\n";
    let mut desktop = Desktop::new(settings)?;
    paper(&desktop)?;
    // An opaque window exactly over another hides it whole, but not its shadow: beside them
    // two shadows lie on top of each other. One alone leaves 217 two pixels from the edge.
    let hidden = desktop.window(rect(100, 60), 0x00ff_0000)?;
    let cover = desktop.window(rect(100, 60), 0x0000_00ff)?;
    desktop.map(hidden)?;
    desktop.map(cover)?;
    desktop.until_pixel((150, 110), |p| p == [0, 0, 255])?;
    let beside = (202, 110);
    desktop.until_pixel(beside, |[r, _, _]| r.abs_diff(185) <= 2)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(&shown, &full_repaint(&desktop, settings)?, "two shadows");
    desktop.conn.unmap_window(hidden)?.check()?;
    desktop.until_pixel(beside, |[r, _, _]| r.abs_diff(217) <= 1)?;
    // A translucent window blurs what is beneath it; hidden whole, it does not blur, and
    // still casts its shadow, at its own opacity.
    desktop.opacity(hidden, 0x8000_0000)?;
    desktop.map(hidden)?;
    desktop.until_pixel(beside, |[r, _, _]| r.abs_diff(201) <= 2)?;
    desktop.wait_vblanks(2)?;
    let shown = desktop.image()?;
    assert_same(
        &shown,
        &full_repaint(&desktop, settings)?,
        "a hidden blur's shadow",
    );
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

#[test]
fn small_resizes_upload_profiles_without_allocating() -> Result<()> {
    let (desktop, mut requests) = Desktop::with_display(DARK, Requests::start)?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((202, 110), |[r, _, _]| r == 179)?;
    desktop.wait_vblanks(2)?;
    let (pixmaps, images) = (
        requests.count(CREATE_PIXMAP_REQUEST),
        requests.count(PUT_IMAGE_REQUEST),
    );
    // Given a window dragged four pixels wider, one at a time, its strips have room for the
    // longer profile: each step uploads the two profiles and creates no buffer.
    for width in 101..=104_u16 {
        let aux = ConfigureWindowAux::new().width(u32::from(width));
        desktop.conn.configure_window(window, &aux)?.check()?;
        let beside = 100 + width.cast_signed() + 2;
        desktop.until_pixel((beside, 110), |[r, _, _]| r == 179)?;
    }
    desktop.wait_vblanks(2)?;
    assert_eq!(requests.count(CREATE_PIXMAP_REQUEST) - pixmaps, 0);
    assert_eq!(requests.count(PUT_IMAGE_REQUEST) - images, 8);
    requests.finish()
}

#[test]
fn shadow_strips_follow_resizes_without_leaking() -> Result<()> {
    let desktop = Desktop::new(DARK)?;
    paper(&desktop)?;
    let window = desktop.window(rect(100, 60), 0x00ff_0000)?;
    desktop.map(window)?;
    let beside = (202, 110);
    desktop.until_pixel(beside, |[r, _, _]| r == 179)?;
    // Each round makes the window narrower and taller and back; the shadow follows both
    // edges. Strips grow to hold the longest profile so far, so the first round warms up.
    let resize = || -> Result<()> {
        let aux = ConfigureWindowAux::new().width(60).height(140);
        desktop.conn.configure_window(window, &aux)?.check()?;
        desktop.until_pixel((162, 110), |[r, _, _]| r == 179)?;
        desktop.until_pixel(beside, |p| p == WHITE)?;
        assert_eq!(gray(&desktop, (130, 202))?, 179);
        let aux = ConfigureWindowAux::new().width(100).height(100);
        desktop.conn.configure_window(window, &aux)?.check()?;
        desktop.until_pixel(beside, |[r, _, _]| r == 179)?;
        desktop.until_pixel((130, 202), |p| p == WHITE)?;
        assert_eq!(gray(&desktop, (150, 162))?, 179);
        desktop.wait_vblanks(2)
    };
    resize()?;
    let baseline = desktop.resources()?;
    for round in 0..4 {
        resize()?;
        assert_eq!(desktop.resources()?, baseline, "round {round}");
    }
    // Without shadows the strips go, and they come back the same.
    desktop.reload("fade_ms = 0\nblur_radius = 0\n")?;
    desktop.until_pixel(beside, |p| p == WHITE)?;
    desktop.wait_vblanks(2)?;
    assert_ne!(desktop.resources()?, baseline);
    desktop.reload(DARK)?;
    desktop.until_pixel(beside, |[r, _, _]| r == 179)?;
    resize()?;
    assert_eq!(desktop.resources()?, baseline);
    Ok(())
}
