use super::proxy::Proxy;
use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

/// Counts the compositor's core protocol requests by major opcode, and extension requests by
/// major and minor opcode.
pub(crate) struct Requests {
    proxy: Proxy,
    counts: Arc<[AtomicU32; 128]>,
    extensions: Arc<Mutex<HashMap<(u8, u8), u32>>>,
}

impl Requests {
    pub(crate) fn start(display: &str) -> Result<(String, Self)> {
        let counts = Arc::new(std::array::from_fn(|_| AtomicU32::new(0)));
        let extensions = Arc::new(Mutex::new(HashMap::new()));
        let shared = Arc::clone(&counts);
        let record = Arc::clone(&extensions);
        let (display, proxy) = Proxy::start(display, move |header, _| {
            if let Some(count) = shared.get(usize::from(header.major_opcode)) {
                count.fetch_add(1, Ordering::Relaxed);
            } else {
                *record
                    .lock()
                    .map_err(|_| anyhow::anyhow!("request counts poisoned"))?
                    .entry((header.major_opcode, header.minor_opcode))
                    .or_default() += 1;
            }
            Ok(())
        })?;
        Ok((
            display,
            Self {
                proxy,
                counts,
                extensions,
            },
        ))
    }

    pub(crate) fn count(&self, opcode: u8) -> u32 {
        self.counts
            .get(usize::from(opcode))
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    /// Requests `minor` of the extension whose major opcode is `major`.
    pub(crate) fn count_extension(&self, major: u8, minor: u8) -> Result<u32> {
        Ok(self
            .extensions
            .lock()
            .map_err(|_| anyhow::anyhow!("request counts poisoned"))?
            .get(&(major, minor))
            .copied()
            .unwrap_or_default())
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.proxy.finish()
    }
}
