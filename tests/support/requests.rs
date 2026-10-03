use super::proxy::Proxy;
use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

/// Counts the compositor's core protocol requests by major opcode.
pub(crate) struct Requests {
    proxy: Proxy,
    counts: Arc<[AtomicU32; 128]>,
}

impl Requests {
    pub(crate) fn start(display: &str) -> Result<(String, Self)> {
        let counts = Arc::new(std::array::from_fn(|_| AtomicU32::new(0)));
        let shared = Arc::clone(&counts);
        let (display, proxy) = Proxy::start(display, move |header, _| {
            if let Some(count) = shared.get(usize::from(header.major_opcode)) {
                count.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        })?;
        Ok((display, Self { proxy, counts }))
    }

    pub(crate) fn count(&self, opcode: u8) -> u32 {
        self.counts
            .get(usize::from(opcode))
            .map_or(0, |count| count.load(Ordering::Relaxed))
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.proxy.finish()
    }
}
