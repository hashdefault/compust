use super::{
    capture::{CapturePoint, Interceptor, Race, Resources},
    proxy::Proxy,
};
use anyhow::{Context, Result};
use std::{
    sync::mpsc::{Receiver, SyncSender, sync_channel},
    time::Duration,
};
use x11rb::protocol::xproto::Window;

pub(crate) struct CaptureProxy {
    proxy: Proxy,
    commands: SyncSender<Race>,
    completed: Receiver<Resources>,
}

impl CaptureProxy {
    pub(crate) fn start(display: &str) -> Result<(String, Self)> {
        let (commands, receiver) = sync_channel(1);
        let (sender, completed) = sync_channel(1);
        let mut interceptor = Interceptor::new(display, receiver, sender)?;
        let (display, proxy) = Proxy::start(display, move |header, body| {
            interceptor.before(header, body)
        })?;
        Ok((
            display,
            Self {
                proxy,
                commands,
                completed,
            },
        ))
    }

    pub(crate) fn arm(&self, window: Window, point: CapturePoint) -> Result<()> {
        self.commands.send(Race::new(window, point))?;
        Ok(())
    }

    pub(crate) fn intercepted(&self) -> Result<Resources> {
        self.completed
            .recv_timeout(Duration::from_secs(5))
            .context("capture request was not intercepted")
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        self.proxy.finish()
    }
}
