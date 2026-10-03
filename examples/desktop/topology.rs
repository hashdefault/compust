use super::Surface;
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        randr::{ConnectionExt as _, ModeFlag, ModeInfo, SetConfig},
        res::{ConnectionExt as _, ResourceIdSpec},
        xproto::{AtomEnum, ConnectionExt as _},
    },
};

/// A pixel position in root coordinates.
pub(super) type Point = (i32, i32);

/// A marker origin straddling a shared monitor edge, with one point on each side.
pub(super) struct Seam {
    pub(super) origin: Point,
    pub(super) points: [Point; 2],
}

/// A rectangle in root coordinates.
pub(super) struct Area {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

/// Write outputs, CRTCs, and active monitors to `topology.txt`; return the monitors.
pub(super) fn capture(surface: &Surface, output: &Path) -> Result<Vec<Area>> {
    let conn = &surface.conn;
    let mut file = BufWriter::new(File::create(output.join("topology.txt"))?);
    let root = conn.get_geometry(surface.root)?.reply()?;
    writeln!(file, "root={}x{}", root.width, root.height)?;
    let resources = conn
        .randr_get_screen_resources_current(surface.root)?
        .reply()?;
    let primary = conn.randr_get_output_primary(surface.root)?.reply()?.output;
    for &id in &resources.outputs {
        let info = conn
            .randr_get_output_info(id, resources.config_timestamp)?
            .reply()?;
        write!(
            file,
            "output={} connection={:?} primary={}",
            String::from_utf8_lossy(&info.name),
            info.connection,
            id == primary
        )?;
        if info.crtc == NONE {
            writeln!(file, " crtc=none")?;
            continue;
        }
        let crtc = conn
            .randr_get_crtc_info(info.crtc, resources.config_timestamp)?
            .reply()?;
        ensure!(crtc.status == SetConfig::SUCCESS, "querying a CRTC failed");
        write!(
            file,
            " crtc={} geometry={}x{}+{}+{} rotation={:?}",
            info.crtc, crtc.width, crtc.height, crtc.x, crtc.y, crtc.rotation
        )?;
        match resources.modes.iter().find(|mode| mode.id == crtc.mode) {
            Some(mode) => {
                let rate = refresh_millihertz(mode);
                writeln!(
                    file,
                    " mode={}x{}@{}.{:03}Hz",
                    mode.width,
                    mode.height,
                    rate / 1000,
                    rate % 1000
                )?;
            }
            None => writeln!(file, " mode=none")?,
        }
    }
    let monitors = conn
        .randr_get_monitors(surface.root, true)?
        .reply()?
        .monitors;
    let mut areas = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let name = conn.get_atom_name(monitor.name)?.reply()?.name;
        writeln!(
            file,
            "monitor={} primary={} automatic={} geometry={}x{}+{}+{} outputs={}",
            String::from_utf8_lossy(&name),
            monitor.primary,
            monitor.automatic,
            monitor.width,
            monitor.height,
            monitor.x,
            monitor.y,
            monitor.outputs.len()
        )?;
        areas.push(Area {
            x: monitor.x.into(),
            y: monitor.y.into(),
            width: monitor.width.into(),
            height: monitor.height.into(),
        });
    }
    file.flush()?;
    Ok(areas)
}

/// Vertical refresh as xrandr reports it, in millihertz; zero for incomplete timings.
fn refresh_millihertz(mode: &ModeInfo) -> u64 {
    let mut lines = u64::from(mode.vtotal);
    if mode.mode_flags.contains(ModeFlag::DOUBLE_SCAN) {
        lines *= 2;
    }
    if mode.mode_flags.contains(ModeFlag::INTERLACE) {
        lines /= 2;
    }
    let pixels = u64::from(mode.htotal) * lines;
    if pixels == 0 {
        return 0;
    }
    u64::from(mode.dot_clock) * 1000 / pixels
}

/// Find every edge shared by two monitors that can hold the straddling marker.
pub(super) fn seams(monitors: &[Area], marker: &Area) -> Vec<Seam> {
    let mut seams = Vec::new();
    for first in monitors {
        for second in monitors {
            let top = first.y.max(second.y);
            let bottom = (first.y + first.height).min(second.y + second.height);
            if first.x + first.width == second.x && bottom - top >= marker.height + 80 {
                let origin = (second.x - marker.width / 2, top + 40);
                let y = origin.1 + marker.height / 2;
                let offset = marker.width / 4;
                seams.push(Seam {
                    origin,
                    points: [(second.x - offset, y), (second.x + offset, y)],
                });
            }
            let left = first.x.max(second.x);
            let right = (first.x + first.width).min(second.x + second.width);
            if first.y + first.height == second.y && right - left >= marker.width + 40 {
                let origin = (left + 20, second.y - marker.height / 2);
                let x = origin.0 + marker.width / 2;
                let offset = marker.height / 4;
                seams.push(Seam {
                    origin,
                    points: [(x, second.y - offset), (x, second.y + offset)],
                });
            }
        }
    }
    seams
}

/// Write the compositor's X resource counts and full owned pixmap bytes to `resources.csv`.
/// Record the compositor's server resources in `resources.csv`. Compust's pixmaps all report
/// their size; with `strict` unset, pixmaps without one are counted instead of rejected, as
/// another compositor's GLX pixmaps can be.
pub(super) fn resources(surface: &Surface, output: &Path, strict: bool) -> Result<()> {
    let conn = &surface.conn;
    let version = conn.res_query_version(1, 2)?.reply()?;
    ensure!(
        (version.server_major, version.server_minor) >= (1, 2),
        "XRes 1.2 is required"
    );
    let screen = conn
        .setup()
        .roots
        .iter()
        .position(|screen| screen.root == surface.root)
        .context("missing screen")?;
    let selection = surface.atom(&format!("_NET_WM_CM_S{screen}"))?;
    // XRes accepts any XID owned by the client, including its selection window.
    let owner = conn.get_selection_owner(selection)?.reply()?.owner;
    ensure!(owner != NONE, "compositor lost its selection");
    let mut file = BufWriter::new(File::create(output.join("resources.csv"))?);
    writeln!(file, "resource,value")?;
    let mut pixmaps = 0;
    for resource in conn.res_query_client_resources(owner)?.reply()?.types {
        let name = conn.get_atom_name(resource.resource_type)?.reply()?.name;
        if resource.resource_type == u32::from(AtomEnum::PIXMAP) {
            pixmaps = resource.count;
        }
        writeln!(file, "{},{}", String::from_utf8(name)?, resource.count)?;
    }
    let mut sizes = conn
        .res_query_resource_bytes(
            owner,
            &[ResourceIdSpec {
                resource: NONE,
                type_: NONE,
            }],
        )?
        .reply()?
        .sizes;
    sizes.retain(|size| size.size.spec.type_ == u32::from(AtomEnum::PIXMAP));
    let reported = u32::try_from(sizes.iter().filter(|size| size.size.bytes > 0).count())?;
    ensure!(
        !strict || u32::try_from(sizes.len())? == pixmaps,
        "XRes omitted pixmap allocations"
    );
    ensure!(!strict || reported == pixmaps, "XRes omitted pixmap bytes");
    if !strict {
        writeln!(
            file,
            "pixmaps_without_size,{}",
            pixmaps.saturating_sub(reported)
        )?;
    }
    // Full allocation sizes stay stable when Present changes pixmap reference counts.
    let bytes = sizes.iter().try_fold(0_u64, |sum, size| {
        sum.checked_add(u64::from(size.size.bytes))
            .context("pixmap byte overflow")
    })?;
    writeln!(file, "owned_pixmap_bytes,{bytes}")?;
    file.flush()?;
    Ok(())
}
