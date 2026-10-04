use crate::{
    rect,
    regions::{area, striped},
    reload::half_red,
    support::{Desktop, proxy::Proxy},
};
use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        Atom, AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as _,
        CreateWindowAux, GET_PROPERTY_REQUEST, PropMode, Rectangle, Window, WindowClass,
    },
    wrapper::ConnectionExt as _,
};

const QUICK: &str = "fade_ms = 0\nblur_radius = 0\n";
const DIM: &str = "[[rules]]\nwm_class = \"Dim\"\nopacity = 50\n";
const RED: [u8; 3] = [255, 0, 0];
const BACKGROUND: [u8; 3] = [24, 24, 32];

pub(crate) fn atom(desktop: &Desktop, name: &[u8]) -> Result<Atom> {
    Ok(desktop.conn.intern_atom(false, name)?.reply()?.atom)
}

/// A red window whose `WM_CLASS` names `class`.
pub(crate) fn classed(desktop: &Desktop, rect: Rectangle, class: &str) -> Result<Window> {
    let window = desktop.window(rect, 0x00ff_0000)?;
    desktop
        .conn
        .change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            format!("instance\0{class}\0").as_bytes(),
        )?
        .check()?;
    Ok(window)
}

pub(crate) fn set_types(desktop: &Desktop, window: Window, types: &[Atom]) -> Result<()> {
    let property = atom(desktop, b"_NET_WM_WINDOW_TYPE")?;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, window, property, AtomEnum::ATOM, types)?
        .check()?;
    Ok(())
}

fn set_transient(desktop: &Desktop, window: Window) -> Result<()> {
    desktop
        .conn
        .change_property32(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_TRANSIENT_FOR,
            AtomEnum::WINDOW,
            &[desktop.root],
        )?
        .check()?;
    Ok(())
}

/// Reparent `client` into `frame` as a window manager does, with `WM_STATE` when `managed`.
pub(crate) fn frame(desktop: &Desktop, frame: Window, client: Window, managed: bool) -> Result<()> {
    desktop
        .conn
        .reparent_window(client, frame, 10, 10)?
        .check()?;
    if managed {
        manage(desktop, client)?;
    }
    desktop.map(client)?;
    desktop.map(frame)
}

fn manage(desktop: &Desktop, client: Window) -> Result<()> {
    let state = atom(desktop, b"WM_STATE")?;
    desktop
        .conn
        .change_property32(PropMode::REPLACE, client, state, state, &[1, 0])?
        .check()?;
    Ok(())
}

#[test]
fn class_rules_replace_the_global_opacity_and_follow_clients_into_frames() -> Result<()> {
    let desktop = Desktop::new(&format!(
        "{QUICK}opacity = 50\n\n[[rules]]\nwm_class = \"Solid\"\nopacity = 100\n"
    ))?;
    let solid = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let client = classed(&desktop, rect(0, 0), "Solid")?;
    frame(&desktop, solid, client, true)?;
    let other = classed(&desktop, rect(160, 20), "Other")?;
    desktop.map(other)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    desktop.until_pixel((180, 40), half_red)?;

    // The client closes. Its frame, which a window manager would now unmap, keeps the
    // client's rule instead of flashing to another opacity as it fades.
    desktop.conn.destroy_window(client)?.check()?;
    desktop.conn.unmap_window(other)?.check()?;
    desktop.until_pixel((180, 40), |p| p == BACKGROUND)?;
    assert_eq!(desktop.pixel((40, 40))?, RED);

    // A frame shown before its client is managed has no class until the client gets WM_STATE.
    let late = desktop.window(rect(160, 120), 0x00ff_0000)?;
    let client = classed(&desktop, rect(0, 0), "Solid")?;
    frame(&desktop, late, client, false)?;
    desktop.until_pixel((180, 140), half_red)?;
    manage(&desktop, client)?;
    desktop.until_pixel((180, 140), |p| p == RED)?;
    Ok(())
}

#[test]
fn windows_keep_their_rules_when_they_close_right_after_a_change() -> Result<()> {
    let desktop = Desktop::new(
        "fade_ms = 1000\nblur_radius = 0\nopacity = 50\n\n\
         [[rules]]\nwm_class = \"Solid\"\nopacity = 100\n",
    )?;
    let window = classed(&desktop, rect(20, 20), "Solid")?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;

    // The compositor reads the new title only after the window is gone.
    let net_wm_name = atom(&desktop, b"_NET_WM_NAME")?;
    let utf8 = atom(&desktop, b"UTF8_STRING")?;
    desktop
        .conn
        .change_property8(PropMode::REPLACE, window, net_wm_name, utf8, b"Closing")?;
    desktop.conn.destroy_window(window)?;
    desktop.conn.flush()?;
    let fading = desktop.until_pixel((40, 40), |[r, _, _]| r < 250)?;
    assert!(
        fading[0] > 150,
        "the closing window lost its rule: {fading:?}"
    );
    desktop.until_pixel((40, 40), |p| p == BACKGROUND)?;
    Ok(())
}

#[test]
fn title_rules_follow_title_changes() -> Result<()> {
    let desktop = Desktop::new(&format!(
        "{QUICK}\n[[rules]]\nname = \"Café\"\nopacity = 50\n"
    ))?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    let net_wm_name = atom(&desktop, b"_NET_WM_NAME")?;
    let utf8 = atom(&desktop, b"UTF8_STRING")?;
    let title = |property: Atom, kind: Atom, text: &[u8]| -> Result<()> {
        desktop
            .conn
            .change_property8(PropMode::REPLACE, window, property, kind, text)?
            .check()?;
        Ok(())
    };
    let wm_name = AtomEnum::WM_NAME.into();
    // _NET_WM_NAME comes before WM_NAME, whose STRING type is Latin-1.
    title(net_wm_name, utf8, b"Bright")?;
    title(wm_name, AtomEnum::STRING.into(), b"Caf\xe9")?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;

    title(net_wm_name, utf8, "Café".as_bytes())?;
    desktop.until_pixel((40, 40), half_red)?;
    title(net_wm_name, utf8, b"Bright")?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    desktop.conn.delete_property(window, net_wm_name)?.check()?;
    desktop.until_pixel((40, 40), half_red)?;
    // A WM_NAME of type UTF8_STRING is UTF-8, so the same bytes are no title at all.
    title(wm_name, utf8, b"Caf\xe9")?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    title(wm_name, utf8, "Café".as_bytes())?;
    desktop.until_pixel((40, 40), half_red)?;
    Ok(())
}

#[test]
fn type_rules_use_the_first_known_type_or_the_ewmh_default() -> Result<()> {
    let desktop = Desktop::new(&format!(
        "{QUICK}\n[[rules]]\nwindow_type = \"dialog\"\nopacity = 50\n"
    ))?;
    let dialog = atom(&desktop, b"_NET_WM_WINDOW_TYPE_DIALOG")?;
    let normal = atom(&desktop, b"_NET_WM_WINDOW_TYPE_NORMAL")?;
    let unknown = atom(&desktop, b"_COMPUST_TEST_UNKNOWN_TYPE")?;
    let column = |index: i16| area(10 + 60 * index, 20, 50, 50);
    let center = |index: i16| (35 + 60 * index, 45);
    let plain = desktop.window(column(0), 0x00ff_0000)?;
    let transient = desktop.window(column(1), 0x00ff_0000)?;
    set_transient(&desktop, transient)?;
    // EWMH makes an override-redirect window without a type normal even when transient.
    let unmanaged = desktop.conn.generate_id()?;
    let place = column(2);
    desktop
        .conn
        .create_window(
            24,
            unmanaged,
            desktop.root,
            place.x,
            place.y,
            place.width,
            place.height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(0x00ff_0000)
                .override_redirect(1),
        )?
        .check()?;
    set_transient(&desktop, unmanaged)?;
    let preferred = desktop.window(column(3), 0x00ff_0000)?;
    set_types(&desktop, preferred, &[unknown, dialog])?;
    let first = desktop.window(column(4), 0x00ff_0000)?;
    set_types(&desktop, first, &[normal, dialog])?;
    for window in [plain, transient, unmanaged, preferred, first] {
        desktop.map(window)?;
    }
    desktop.until_pixel(center(1), half_red)?;
    desktop.until_pixel(center(3), half_red)?;
    desktop.wait_vblanks(2)?;
    for index in [0, 2, 4] {
        assert_eq!(desktop.pixel(center(index))?, RED, "column {index}");
    }

    // Changing the type or the transience applies at once.
    set_transient(&desktop, plain)?;
    desktop.until_pixel(center(0), half_red)?;
    set_types(&desktop, first, &[dialog])?;
    desktop.until_pixel(center(4), half_red)?;
    set_types(&desktop, preferred, &[normal])?;
    desktop.until_pixel(center(3), |p| p == RED)?;
    Ok(())
}

#[test]
fn blur_rules_keep_backdrops_sharp_and_reload_with_the_rest() -> Result<()> {
    let blur = "fade_ms = 0\nblur_radius = 4\n";
    let desktop = Desktop::new(&format!(
        "{blur}\n[[rules]]\nwm_class = \"Frosted\"\nblur = true\n\n\
         [[rules]]\nwindow_type = \"menu\"\nblur = false\n"
    ))?;
    striped(&desktop, 320)?;
    let menu_type = atom(&desktop, b"_NET_WM_WINDOW_TYPE_MENU")?;
    // Half-transparent red windows: a menu, a menu whose class keeps blur on, and a plain one.
    for (x, class, menu) in [
        (20, None, true),
        (120, Some("Frosted"), true),
        (220, None, false),
    ] {
        let place = area(x, 70, 80, 100);
        let window = match class {
            Some(class) => classed(&desktop, place, class)?,
            None => desktop.window(place, 0x00ff_0000)?,
        };
        if menu {
            set_types(&desktop, window, &[menu_type])?;
        }
        desktop.opacity(window, 0x8000_0000)?;
        desktop.map(window)?;
    }
    // Over a black stripe, a sharp backdrop leaves half-strength red; a blurred one shows
    // through as gray, half of it at this opacity.
    let sharp = |[r, g, b]: [u8; 3]| (126..=129).contains(&r) && g == 0 && b == 0;
    let blurred = |[r, g, b]: [u8; 3]| (150..=170).contains(&r) && (16..=48).contains(&g) && g == b;
    desktop.until_pixel((60, 120), sharp)?;
    desktop.until_pixel((160, 120), blurred)?;
    desktop.until_pixel((260, 120), blurred)?;

    desktop.reload(&format!(
        "{blur}\n[[rules]]\nwm_class = \"Frosted\"\nblur = false\n"
    ))?;
    desktop.until_pixel((60, 120), blurred)?;
    desktop.until_pixel((160, 120), sharp)?;
    assert!(blurred(desktop.pixel((260, 120))?));
    Ok(())
}

#[test]
fn fade_rules_time_the_fades_of_matching_windows() -> Result<()> {
    let desktop = Desktop::new(&format!(
        "{QUICK}\n[[rules]]\nwm_class = \"Slow\"\nfade_ms = 1000\n"
    ))?;
    let slow = classed(&desktop, rect(20, 20), "Slow")?;
    desktop.map(slow)?;
    desktop.until_pixel((40, 40), |[r, _, _]| (40..=200).contains(&r))?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    desktop.conn.unmap_window(slow)?.check()?;
    desktop.until_pixel((40, 40), |[r, _, _]| (40..=200).contains(&r))?;

    // Mapped again while it closes, with new content, it fades back in at its own pace.
    desktop
        .conn
        .change_window_attributes(
            slow,
            &ChangeWindowAttributesAux::new().background_pixel(0x0000_ff00),
        )?
        .check()?;
    desktop.map(slow)?;
    let reopened = desktop.until_pixel((40, 40), |[_, g, _]| g > 24)?;
    assert!(
        reopened[1] < 255,
        "the reopening skipped its fade: {reopened:?}"
    );
    desktop.until_pixel((40, 40), |p| p == [0, 255, 0])?;
    desktop.conn.unmap_window(slow)?.check()?;
    desktop.until_pixel((40, 40), |[_, g, _]| (40..=200).contains(&g))?;
    desktop.until_pixel((40, 40), |p| p == BACKGROUND)?;
    Ok(())
}

#[test]
fn reloads_apply_rules_to_open_windows() -> Result<()> {
    let desktop = Desktop::new(QUICK)?;
    let window = classed(&desktop, rect(20, 20), "Dim")?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;

    desktop.reload(&format!("{QUICK}\n{DIM}"))?;
    desktop.until_pixel((40, 40), half_red)?;
    desktop.reload(QUICK)?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    Ok(())
}

#[test]
fn resizes_keep_identities_that_title_changes_read_again() -> Result<()> {
    let reads = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&reads);
    let settings = format!("{QUICK}\n[[rules]]\nname = \"Bright\"\nopacity = 100\n\n{DIM}");
    let (desktop, mut proxy) = Desktop::with_display(&settings, move |display| {
        Proxy::start(display, move |header, body| {
            let class = u32::from(AtomEnum::WM_CLASS).to_ne_bytes();
            if header.major_opcode == GET_PROPERTY_REQUEST && body.get(4..8) == Some(&class[..]) {
                counter.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        })
    })?;
    let window = classed(&desktop, rect(20, 20), "Dim")?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), half_red)?;
    let before = reads.load(Ordering::Relaxed);

    for width in [120_u16, 140, 160, 180] {
        desktop
            .conn
            .configure_window(window, &ConfigureWindowAux::new().width(u32::from(width)))?
            .check()?;
        desktop.until_pixel((15 + i16::try_from(width)?, 40), half_red)?;
    }
    assert_eq!(
        reads.load(Ordering::Relaxed),
        before,
        "a resize read the class again"
    );

    let net_wm_name = atom(&desktop, b"_NET_WM_NAME")?;
    let utf8 = atom(&desktop, b"UTF8_STRING")?;
    desktop
        .conn
        .change_property8(PropMode::REPLACE, window, net_wm_name, utf8, b"Bright")?
        .check()?;
    desktop.conn.flush()?;
    desktop.until_pixel((40, 40), |p| p == RED)?;
    assert_eq!(reads.load(Ordering::Relaxed), before + 1);
    proxy.finish()
}
