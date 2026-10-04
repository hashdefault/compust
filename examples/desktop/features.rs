#[path = "features/fullscreen.rs"]
mod fullscreen;
#[path = "features/scene.rs"]
mod scene;
#[path = "features/shadows.rs"]
mod shadows;

use crate::{Args, metrics::Process, surface::Surface, topology};
use anyhow::{Result, ensure};
use scene::{RED, WHITE, activate, create, typed};
use x11rb::{
    NONE,
    protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, EventMask, PropMode, Rectangle,
    },
    wrapper::ConnectionExt as _,
};

#[derive(clap::Args)]
pub(super) struct Options {
    /// Check shadows, focus rules, and fullscreen suspension without a window manager.
    /// Use tools/desktop/compust-features.toml and label the compositor 'compust'.
    #[arg(long = "features", conflicts_with_all = ["bench", "snapshot", "hotplug"])]
    pub(super) enabled: bool,
    /// Compare the feature scene images with a previous renderer's report directory.
    #[arg(long, requires = "enabled")]
    pub(super) reference: Option<std::path::PathBuf>,
}

pub(super) fn run(surface: &Surface, args: &Args, processes: &[Process]) -> Result<()> {
    ensure!(
        surface.width >= 320 && surface.height >= 240,
        "feature scenes require at least 320x240"
    );
    ensure!(
        surface
            .property(surface.root, "_NET_SUPPORTING_WM_CHECK")?
            .is_none(),
        "feature scenes require an isolated display without a window manager"
    );
    surface
        .conn
        .change_window_attributes(
            surface.overlay,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::STRUCTURE_NOTIFY),
        )?
        .check()?;
    surface
        .conn
        .delete_property(surface.root, surface.atom("_NET_ACTIVE_WINDOW")?)?
        .check()?;
    topology::capture(surface, &args.output)?;
    let paper = create(
        surface,
        Rectangle {
            x: 0,
            y: 0,
            width: surface.width,
            height: surface.height,
        },
        WHITE,
    )?;
    typed(surface, paper, "_NET_WM_WINDOW_TYPE_DESKTOP")?;
    surface.conn.map_window(paper)?.check()?;
    shadows::check(surface, args)?;
    focus(surface, args)?;
    fullscreen::check(surface, args, processes)?;
    topology::resources(surface, &args.output, true)?;
    println!("PASS: shadow pixels, focus rules, fullscreen suspension and resume");
    Ok(())
}

fn focus(surface: &Surface, args: &Args) -> Result<()> {
    let mut windows = Vec::with_capacity(2);
    for x in [20, 160] {
        let window = create(
            surface,
            Rectangle {
                x,
                y: 20,
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
                b"compust-feature\0CompustFeatureFocus\0",
            )?
            .check()?;
        surface.conn.map_window(window)?.check()?;
        windows.push(window);
    }
    let check = |left, right| {
        surface.until("focus rule pixels", || {
            Ok(scene::near(surface.pixel((70, 70))?, left)
                && scene::near(surface.pixel((210, 70))?, right))
        })
    };
    check(RED, RED)?;
    for (index, name, colors) in [
        (0, "focus-left", (RED, 0x00ff_7f7f)),
        (1, "focus-right", (0x00ff_7f7f, RED)),
    ] {
        activate(
            surface,
            *windows
                .get(index)
                .ok_or_else(|| anyhow::anyhow!("missing focus window"))?,
        )?;
        check(colors.0, colors.1)?;
        shadows::snapshot(surface, args, name)?;
    }
    activate(surface, NONE)?;
    check(0x00ff_7f7f, 0x00ff_7f7f)?;
    surface
        .conn
        .delete_property(surface.root, surface.atom("_NET_ACTIVE_WINDOW")?)?
        .check()?;
    check(RED, RED)?;
    for window in windows {
        surface.conn.destroy_window(window)?.check()?;
    }
    println!("PASS: focus switches, no active window, absent active-window property");
    Ok(())
}
