use super::Desktop;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
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

impl Desktop {
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
