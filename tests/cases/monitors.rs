use crate::support::{Desktop, monitors::Monitor};
use anyhow::Result;
use x11rb::protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt as _, Rectangle, Window};

#[test]
fn monitors_resize_root_with_present() -> Result<()> {
    repeated_root_resize(true)
}

#[test]
fn monitors_resize_root_with_xrender() -> Result<()> {
    repeated_root_resize(false)
}

#[test]
fn monitors_toggle_output_with_present() -> Result<()> {
    toggle_output(true)
}

#[test]
fn monitors_toggle_output_with_xrender() -> Result<()> {
    toggle_output(false)
}

fn scene(vsync: bool) -> Result<(Desktop, Monitor, Window)> {
    let desktop = Desktop::new(&format!("fade_ms = 0\nblur_radius = 0\nvsync = {vsync}"))?;
    let monitor = Monitor::new(&desktop)?;
    let window = desktop.window(
        Rectangle {
            x: 20,
            y: 20,
            width: 300,
            height: 220,
        },
        0x00ff_0000,
    )?;
    desktop.map(window)?;
    desktop.until_pixel((310, 230), |p| p == [255, 0, 0])?;
    Ok((desktop, monitor, window))
}

fn paint(desktop: &Desktop, window: Window, color: u32) -> Result<()> {
    desktop
        .conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().background_pixel(color),
        )?
        .check()?;
    desktop
        .conn
        .clear_area(false, window, 0, 0, 0, 0)?
        .check()?;
    Ok(())
}

fn resize_cycle(desktop: &Desktop, monitor: &Monitor, window: Window) -> Result<()> {
    monitor.disable(desktop)?;
    desktop.resize_root((240, 180))?;
    paint(desktop, window, 0x0000_00ff)?;
    desktop.until_pixel((230, 170), |p| p == [0, 0, 255])?;
    assert_eq!(desktop.pixel((5, 5))?, [24, 24, 32]);
    desktop.resize_root((320, 240))?;
    monitor.restore(desktop)?;
    paint(desktop, window, 0x0000_ff00)?;
    desktop.until_pixel((310, 230), |p| p == [0, 255, 0])?;
    paint(desktop, window, 0x00ff_0000)?;
    desktop.until_pixel((310, 230), |p| p == [255, 0, 0])?;
    assert_eq!(desktop.pixel((40, 40))?, [255, 0, 0]);
    Ok(())
}

fn repeated_root_resize(vsync: bool) -> Result<()> {
    let (mut desktop, monitor, window) = scene(vsync)?;
    for _ in 0..4 {
        resize_cycle(&desktop, &monitor, window)?;
    }
    let baseline = desktop.resources()?;
    #[cfg(target_os = "linux")]
    let rss_baseline = desktop.compositor_rss_kib()?;
    desktop.screenshot(&format!("monitors-resize-{vsync}-before"))?;

    for cycle in 0..16 {
        resize_cycle(&desktop, &monitor, window)?;
        assert_eq!(
            desktop.resources()?,
            baseline,
            "root resize {cycle}, vsync={vsync}"
        );
    }
    monitor.disable(&desktop)?;
    desktop.resize_root((240, 180))?;
    paint(&desktop, window, 0x0000_ff00)?;
    desktop.until_pixel((230, 170), |p| p == [0, 255, 0])?;
    paint(&desktop, window, 0x0000_00ff)?;
    desktop.until_pixel((230, 170), |p| p == [0, 0, 255])?;
    assert_ne!(desktop.resources()?, baseline, "root buffers must resize");
    desktop.screenshot(&format!("monitors-resize-{vsync}-small"))?;
    desktop.resize_root((320, 240))?;
    monitor.restore(&desktop)?;
    paint(&desktop, window, 0x0000_ff00)?;
    desktop.until_pixel((310, 230), |p| p == [0, 255, 0])?;
    paint(&desktop, window, 0x00ff_0000)?;
    desktop.until_pixel((310, 230), |p| p == [255, 0, 0])?;
    let final_resources = desktop.resources()?;
    assert_eq!(final_resources, baseline);
    eprintln!("root resize vsync={vsync}: baseline={baseline:?}, final={final_resources:?}");
    #[cfg(target_os = "linux")]
    eprintln!(
        "root resize vsync={vsync}: RSS KiB baseline={rss_baseline}, final={}",
        desktop.compositor_rss_kib()?
    );
    desktop.screenshot(&format!("monitors-resize-{vsync}-after"))?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}

fn toggle_output(vsync: bool) -> Result<()> {
    let (mut desktop, monitor, window) = scene(vsync)?;
    desktop.screenshot(&format!("monitors-output-{vsync}-before"))?;

    monitor.disable(&desktop)?;
    paint(&desktop, window, 0x0000_00ff)?;
    desktop.until_pixel((310, 230), |p| p == [0, 0, 255])?;
    let geometry = desktop.conn.get_geometry(desktop.root)?.reply()?;
    assert_eq!((geometry.width, geometry.height), (320, 240));
    desktop.screenshot(&format!("monitors-output-{vsync}-disabled"))?;
    monitor.restore(&desktop)?;
    paint(&desktop, window, 0x0000_ff00)?;

    desktop.until_pixel((310, 230), |p| p == [0, 255, 0])?;
    assert_eq!(desktop.pixel((40, 40))?, [0, 255, 0]);
    desktop.screenshot(&format!("monitors-output-{vsync}-after"))?;
    assert!(desktop.compositor.0.try_wait()?.is_none());
    Ok(())
}
