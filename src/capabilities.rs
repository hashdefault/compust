use anyhow::{Result, ensure};
use std::{cell::Cell, io::Write};
use x11rb::{
    connection::RequestConnection,
    protocol::{composite, damage, dri3, present, randr, render, shape, sync, xfixes},
    rust_connection::RustConnection,
};

#[derive(Debug)]
pub(crate) struct Capabilities {
    versions: Vec<(&'static str, Option<(u32, u32)>)>,
    pub(crate) present: bool,
    pub(crate) randr: bool,
    pub(crate) bilinear: bool,
    /// Whether the GPU renderer can run; a renderer that fails marks it so for the session.
    pub(crate) gpu: Cell<Gpu>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gpu {
    /// DRI3 1.2 shares pixmaps' buffers, with their modifiers.
    Usable,
    /// The server lacks DRI3 1.2.
    Missing,
    /// The GPU renderer failed; the session keeps to `XRender`.
    Failed,
}

impl Capabilities {
    pub(crate) fn query(conn: &RustConnection, root: u32) -> Result<Self> {
        let mut versions = Vec::new();
        for name in [
            "Composite",
            "DAMAGE",
            "RENDER",
            "XFIXES",
            "SHAPE",
            "RANDR",
            "Present",
            "DRI3",
            "SYNC",
        ] {
            let version = if conn.extension_information(name)?.is_some() {
                Some(match name {
                    "Composite" => {
                        let v = composite::query_version(conn, 0, 4)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "DAMAGE" => {
                        let v = damage::query_version(conn, 1, 1)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "RENDER" => {
                        let v = render::query_version(conn, 0, 11)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "XFIXES" => {
                        let v = xfixes::query_version(conn, 5, 0)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "SHAPE" => {
                        let v = shape::query_version(conn)?.reply()?;
                        (u32::from(v.major_version), u32::from(v.minor_version))
                    }
                    "RANDR" => {
                        let v = randr::query_version(conn, 1, 5)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "Present" => {
                        let v = present::query_version(conn, 1, 2)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    "DRI3" => {
                        let v = dri3::query_version(conn, 1, 2)?.reply()?;
                        (v.major_version, v.minor_version)
                    }
                    _ => {
                        let v = sync::initialize(conn, 3, 1)?.reply()?;
                        (u32::from(v.major_version), u32::from(v.minor_version))
                    }
                })
            } else {
                None
            };
            versions.push((name, version));
        }
        let has = |name| versions.iter().any(|(n, v)| *n == name && v.is_some());
        let bilinear = if has("RENDER") {
            render::query_filters(conn, root)?
                .reply()?
                .filters
                .iter()
                .any(|f| f.name == b"bilinear")
        } else {
            false
        };
        let dri3 = versions
            .iter()
            .any(|(name, version)| *name == "DRI3" && version.is_some_and(|v| v >= (1, 2)));
        Ok(Self {
            present: has("Present"),
            randr: has("RANDR"),
            bilinear,
            gpu: Cell::new(if dri3 { Gpu::Usable } else { Gpu::Missing }),
            versions,
        })
    }

    pub(crate) fn require_baseline(&self) -> Result<()> {
        for (name, minimum) in [
            ("Composite", (0, 4)),
            ("DAMAGE", (1, 0)),
            ("RENDER", (0, 11)),
            ("XFIXES", (2, 0)),
            ("SHAPE", (1, 1)),
        ] {
            ensure!(
                self.versions
                    .iter()
                    .any(|(n, v)| *n == name && v.is_some_and(|v| v >= minimum)),
                "server requires {name} {}.{} or newer",
                minimum.0,
                minimum.1
            );
        }
        Ok(())
    }

    pub(crate) fn report(&self) -> Result<()> {
        let mut out = std::io::stdout().lock();
        for (name, version) in &self.versions {
            match version {
                Some((major, minor)) => writeln!(out, "{name}: {major}.{minor}")?,
                None => writeln!(out, "{name}: unavailable")?,
            }
        }
        writeln!(out, "XRender bilinear blur: {}", self.bilinear)?;
        let gpu = match self.gpu.get() {
            Gpu::Usable => "DRI3 1.2 shares pixmaps with the GPU renderer (backend = \"gl\")",
            Gpu::Missing | Gpu::Failed => "unavailable; the server lacks DRI3 1.2",
        };
        writeln!(out, "GPU renderer: {gpu}")?;
        writeln!(
            out,
            "Sync is a diagnostic probe only; frames rely on implicit synchronization."
        )?;
        Ok(())
    }
}
