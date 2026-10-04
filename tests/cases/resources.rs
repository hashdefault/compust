use crate::{rect, support::Desktop};
use anyhow::{Result, ensure};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        res::{ConnectionExt as _, ResourceIdSpec},
        xproto::{AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateGCAux, Window},
    },
};

#[test]
fn resources_stay_stable_when_mapped_window_resizes_with_present() -> Result<()> {
    repeated_resize(true)
}

#[test]
fn resources_stay_stable_when_mapped_window_resizes_with_xrender() -> Result<()> {
    repeated_resize(false)
}

#[test]
fn resources_stay_stable_when_windows_map_and_destroy_with_present() -> Result<()> {
    repeated_map_destroy(true)
}

#[test]
fn resources_stay_stable_when_windows_map_and_destroy_with_xrender() -> Result<()> {
    repeated_map_destroy(false)
}

#[test]
fn resources_stay_stable_when_blurred_windows_change_with_present() -> Result<()> {
    repeated_blurred_changes(true)
}

#[test]
fn resources_stay_stable_when_blurred_windows_change_with_xrender() -> Result<()> {
    repeated_blurred_changes(false)
}

#[test]
fn allocation_bytes_ignore_other_clients_pixmap_references() -> Result<()> {
    let desktop = Desktop::new("fade_ms = 0\nblur_radius = 0\nvsync = false")?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |pixel| pixel == [255, 0, 0])?;
    let baseline = desktop.resources()?;
    let selection = desktop
        .conn
        .intern_atom(false, b"_NET_WM_CM_S0")?
        .reply()?
        .atom;
    let owner = desktop.conn.get_selection_owner(selection)?.reply()?.owner;
    let before = desktop.conn.res_query_client_pixmap_bytes(owner)?.reply()?;
    let sizes = desktop
        .conn
        .res_query_resource_bytes(
            owner,
            &[ResourceIdSpec {
                resource: NONE,
                type_: NONE,
            }],
        )?
        .reply()?
        .sizes;
    let mut references = Vec::new();
    for size in sizes {
        let spec = size.size.spec;
        if spec.type_ == u32::from(AtomEnum::PIXMAP)
            && desktop.conn.get_geometry(spec.resource)?.reply()?.depth == 24
        {
            let gc = desktop.conn.generate_id()?;
            desktop
                .conn
                .create_gc(gc, desktop.root, &CreateGCAux::new().tile(spec.resource))?
                .check()?;
            references.push(gc);
        }
    }
    ensure!(references.len() >= 2, "expected root buffer pixmaps");
    let held = desktop.resources()?;
    let after = desktop.conn.res_query_client_pixmap_bytes(owner)?.reply()?;
    let attributed_before = (u64::from(before.bytes_overflow) << 32) | u64::from(before.bytes);
    let attributed_after = (u64::from(after.bytes_overflow) << 32) | u64::from(after.bytes);
    assert_eq!(
        held, baseline,
        "extra references changed owned allocation bytes"
    );
    assert!(
        attributed_after < attributed_before,
        "extra references did not change attribution"
    );
    eprintln!(
        "reference accounting: full={held:?}, attributed={attributed_before}->{attributed_after}"
    );
    for gc in references {
        desktop.conn.free_gc(gc)?.check()?;
    }
    Ok(())
}

fn repeated_resize(vsync: bool) -> Result<()> {
    let mut desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = 0\nvsync = {vsync}"))?;
    let window = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(window)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    for _ in 0..4 {
        resize_cycle(&desktop, window)?;
    }
    let baseline = desktop.resources()?;
    #[cfg(target_os = "linux")]
    let rss_baseline = desktop.compositor_rss_kib()?;
    desktop.screenshot(&format!("resources-resize-{vsync}-before"))?;

    for cycle in 0..32 {
        resize_cycle(&desktop, window)?;
        assert_eq!(
            desktop.resources()?,
            baseline,
            "resize cycle {cycle}, vsync={vsync}"
        );
    }
    let final_resources = desktop.resources()?;
    eprintln!("resize vsync={vsync}: baseline={baseline:?}, final={final_resources:?}");
    #[cfg(target_os = "linux")]
    eprintln!(
        "resize vsync={vsync}: RSS KiB baseline={rss_baseline}, final={}",
        desktop.compositor_rss_kib()?
    );
    desktop.screenshot(&format!("resources-resize-{vsync}-after"))?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

fn resize_cycle(desktop: &Desktop, window: Window) -> Result<()> {
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().width(260).height(180))?
        .check()?;
    desktop.until_pixel((260, 180), |p| p == [255, 0, 0])?;
    desktop
        .conn
        .configure_window(window, &ConfigureWindowAux::new().width(100).height(100))?
        .check()?;
    desktop.until_pixel((260, 180), |p| p == [24, 24, 32])?;
    assert_eq!(desktop.pixel((40, 40))?, [255, 0, 0]);
    Ok(())
}

fn repeated_map_destroy(vsync: bool) -> Result<()> {
    let mut desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = 0\nvsync = {vsync}"))?;
    let lower = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.map(lower)?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    for _ in 0..4 {
        map_destroy_cycle(&desktop)?;
    }
    let baseline = desktop.resources()?;
    #[cfg(target_os = "linux")]
    let rss_baseline = desktop.compositor_rss_kib()?;
    desktop.screenshot(&format!("resources-map-destroy-{vsync}-before"))?;

    for cycle in 0..32 {
        map_destroy_cycle(&desktop)?;
        assert_eq!(
            desktop.resources()?,
            baseline,
            "map/destroy cycle {cycle}, vsync={vsync}"
        );
    }
    let final_resources = desktop.resources()?;
    eprintln!("map/destroy vsync={vsync}: baseline={baseline:?}, final={final_resources:?}");
    #[cfg(target_os = "linux")]
    eprintln!(
        "map/destroy vsync={vsync}: RSS KiB baseline={rss_baseline}, final={}",
        desktop.compositor_rss_kib()?
    );
    desktop.screenshot(&format!("resources-map-destroy-{vsync}-after"))?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

fn map_destroy_cycle(desktop: &Desktop) -> Result<()> {
    let transient = desktop.window(rect(20, 20), 0x0000_00ff)?;
    desktop.map(transient)?;
    desktop.until_pixel((40, 40), |p| p == [0, 0, 255])?;
    desktop.conn.destroy_window(transient)?.check()?;
    desktop.until_pixel((40, 40), |p| p == [255, 0, 0])?;
    Ok(())
}

/// Kept blur backdrops grow, shrink, and go with their windows.
fn repeated_blurred_changes(vsync: bool) -> Result<()> {
    let mut desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = 4\nvsync = {vsync}"))?;
    let resized = desktop.window(rect(20, 20), 0x00ff_0000)?;
    desktop.opacity(resized, 0x8000_0000)?;
    desktop.map(resized)?;
    desktop.until_pixel((40, 40), reddish)?;
    for _ in 0..4 {
        blurred_cycle(&desktop, resized)?;
    }
    let baseline = desktop.resources()?;
    for cycle in 0..32 {
        blurred_cycle(&desktop, resized)?;
        assert_eq!(
            desktop.resources()?,
            baseline,
            "blurred cycle {cycle}, vsync={vsync}"
        );
    }
    eprintln!("blurred vsync={vsync}: baseline={baseline:?}");
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

fn blurred_cycle(desktop: &Desktop, resized: Window) -> Result<()> {
    let background = [24, 24, 32];
    for (size, expected) in [((260, 180), true), ((100, 100), false)] {
        desktop
            .conn
            .configure_window(
                resized,
                &ConfigureWindowAux::new().width(size.0).height(size.1),
            )?
            .check()?;
        desktop.until_pixel((260, 180), |p| {
            reddish(p) == expected && (expected || p == background)
        })?;
    }
    let transient = desktop.window(rect(150, 20), 0x0000_00ff)?;
    desktop.opacity(transient, 0x8000_0000)?;
    desktop.map(transient)?;
    desktop.until_pixel((170, 40), |[r, _, b]| b > 100 && r < 60)?;
    desktop.conn.destroy_window(transient)?.check()?;
    desktop.until_pixel((170, 40), |p| p == background)?;
    Ok(())
}

fn reddish([r, g, b]: [u8; 3]) -> bool {
    r > 100 && g < 60 && b < 60
}
