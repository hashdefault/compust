use super::Desktop;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::{fs::File, io::Write, time::Instant};
use x11rb::{
    NONE,
    protocol::{
        res::{ConnectionExt as _, ResourceIdSpec},
        xproto::{AtomEnum, ConnectionExt as _},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Resources {
    counts: BTreeMap<String, u32>,
    pixmap_bytes: u64,
}

impl Resources {
    pub(crate) fn pixmap_bytes(&self) -> u64 {
        self.pixmap_bytes
    }

    /// How many resources of each type the compositor owns, without their sizes.
    pub(crate) fn counts(&self) -> &BTreeMap<String, u32> {
        &self.counts
    }
}

pub(crate) struct ResourceProgress {
    name: &'static str,
    started: Instant,
    csv: Option<File>,
}

impl ResourceProgress {
    pub(crate) fn new(name: &'static str) -> Result<Self> {
        let csv = std::env::var_os("COMPUST_ARTIFACTS")
            .map(|directory| {
                std::fs::create_dir_all(&directory)?;
                let mut file =
                    File::create(std::path::PathBuf::from(directory).join(format!("{name}.csv")))?;
                writeln!(file, "cycle,elapsed_ms,rss_kib,pixmap_bytes,counts")?;
                Ok::<_, anyhow::Error>(file)
            })
            .transpose()?;
        Ok(Self {
            name,
            started: Instant::now(),
            csv,
        })
    }

    pub(crate) fn checkpoint(&mut self, desktop: &Desktop, cycle: usize) -> Result<()> {
        let resources = desktop.resources()?;
        let elapsed = self.started.elapsed();
        #[cfg(target_os = "linux")]
        let rss = desktop.compositor_rss_kib()?;
        #[cfg(not(target_os = "linux"))]
        let rss = 0;
        eprintln!(
            "{} cycle={cycle} elapsed={elapsed:?} resources={resources:?} rss_kib={rss}",
            self.name
        );
        if let Some(csv) = &mut self.csv {
            let counts = format!("{:?}", resources.counts()).replace('"', "\"\"");
            writeln!(
                csv,
                "{cycle},{},{rss},{},\"{counts}\"",
                elapsed.as_millis(),
                resources.pixmap_bytes()
            )?;
            csv.flush()?;
        }
        Ok(())
    }
}

impl Desktop {
    /// Wait for fade snapshots and held captures to release their server resources.
    pub(crate) fn until_resources(&self, baseline: &Resources) -> Result<Resources> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let current = self.resources()?;
            if current == *baseline || std::time::Instant::now() >= deadline {
                return Ok(current);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    pub(crate) fn resources(&self) -> Result<Resources> {
        let version = self.conn.res_query_version(1, 2)?.reply()?;
        ensure!(
            (version.server_major, version.server_minor) >= (1, 2),
            "resource accounting requires XRes 1.2"
        );
        let selection = self
            .conn
            .intern_atom(false, b"_NET_WM_CM_S0")?
            .reply()?
            .atom;
        // XRes accepts any XID owned by the client, including its selection window.
        let owner = self.conn.get_selection_owner(selection)?.reply()?.owner;
        ensure!(owner != NONE, "compositor lost its selection");
        let mut counts = BTreeMap::new();
        for resource in self.conn.res_query_client_resources(owner)?.reply()?.types {
            let name = self
                .conn
                .get_atom_name(resource.resource_type)?
                .reply()?
                .name;
            if resource.count > 0 {
                counts.insert(String::from_utf8(name)?, resource.count);
            }
        }
        let mut sizes = self
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
        sizes.retain(|value| value.size.spec.type_ == u32::from(AtomEnum::PIXMAP));
        ensure!(
            u32::try_from(sizes.len())? == counts.get("PIXMAP").copied().unwrap_or(0),
            "XRes omitted sizes for compositor pixmaps"
        );
        ensure!(
            sizes.iter().all(|value| value.size.bytes > 0),
            "XRes omitted allocation bytes for compositor pixmaps"
        );
        // Full allocation sizes stay stable when Present changes pixmap reference counts.
        let pixmap_bytes = sizes.iter().try_fold(0_u64, |total, value| {
            total
                .checked_add(u64::from(value.size.bytes))
                .context("compositor pixmap allocation total overflowed")
        })?;
        Ok(Resources {
            counts,
            pixmap_bytes,
        })
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn compositor_rss_kib(&self) -> Result<u64> {
        let status = std::fs::read_to_string(format!("/proc/{}/status", self.compositor.0.id()))?;
        status
            .lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|line| line.split_whitespace().next())
            .context("compositor status lacks VmRSS")?
            .parse()
            .context("invalid compositor VmRSS")
    }
}
