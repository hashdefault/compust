use crate::{
    rect,
    support::{
        Desktop,
        capture::{CapturePoint, Resources},
        capture_proxy::CaptureProxy,
    },
};
use anyhow::{Context, Result, bail};
use x11rb::{
    NONE,
    errors::ReplyError,
    protocol::{
        ErrorKind,
        damage::ConnectionExt as _,
        render::{ChangePictureAux, ConnectionExt as _},
        xproto::ConnectionExt as _,
    },
};

#[test]
fn survives_destruction_before_naming_window_pixmap() -> Result<()> {
    exercise(&[
        CapturePoint::Attributes,
        CapturePoint::Geometry,
        CapturePoint::Pixmap,
    ])
}

#[test]
fn releases_picture_when_window_dies_during_client_discovery() -> Result<()> {
    exercise(&[
        CapturePoint::Picture,
        CapturePoint::ClientEvents,
        CapturePoint::ClientState,
        CapturePoint::ClientTree,
        CapturePoint::ShapeEvents,
    ])
}

#[test]
fn releases_damage_when_window_dies_during_surface_initialization() -> Result<()> {
    exercise(&[
        CapturePoint::Damage,
        CapturePoint::Shape,
        CapturePoint::Opacity,
    ])
}

fn exercise(points: &[CapturePoint]) -> Result<()> {
    let (mut desktop, mut proxy) =
        Desktop::with_display("fade_ms = 0\nblur_radius = 0", CaptureProxy::start)?;
    let lower = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(lower)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;

    for &point in points {
        let result = (|| -> Result<()> {
            let doomed = desktop.window(rect(20, 20), 0x00ff_0000)?;
            let marker = desktop.window(rect(180, 20), 0x0000_ff00)?;
            proxy.arm(doomed, point)?;
            desktop.map(doomed)?;
            let resources = proxy.intercepted()?;

            desktop.map(marker)?;
            desktop.until_pixel((200, 40), |p| p == [0, 255, 0])?;
            assert_eq!(desktop.pixel((40, 40))?, [0, 0, 255]);
            assert!(desktop.compositor.0.try_wait()?.is_none());
            released(&desktop, resources)?;
            desktop.screenshot(&format!("capture-race-{point:?}"))?;

            desktop.conn.destroy_window(marker)?.check()?;
            desktop.until_pixel((200, 40), |p| p == [24, 24, 32])?;
            Ok(())
        })();
        result.with_context(|| format!("destruction before {point:?}"))?;
    }
    proxy.finish()
}

fn released(desktop: &Desktop, resources: Resources) -> Result<()> {
    if let Some(pixmap) = resources.pixmap {
        protocol_error(
            desktop.conn.get_geometry(pixmap)?.reply(),
            ErrorKind::Drawable,
        )?;
    }
    if let Some(picture) = resources.picture {
        protocol_error(
            desktop
                .conn
                .render_change_picture(picture, &ChangePictureAux::default())?
                .check(),
            ErrorKind::RenderPicture,
        )?;
    }
    if let Some(damage) = resources.damage {
        protocol_error(
            desktop.conn.damage_subtract(damage, NONE, NONE)?.check(),
            ErrorKind::DamageBadDamage,
        )?;
    }
    Ok(())
}

fn protocol_error<T>(
    result: std::result::Result<T, ReplyError>,
    expected: ErrorKind,
) -> Result<()> {
    match result {
        Err(ReplyError::X11Error(error)) if error.error_kind == expected => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => bail!("capture resource remained allocated; expected {expected:?}"),
    }
}
