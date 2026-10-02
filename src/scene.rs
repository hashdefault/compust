use crate::{
    session::Session,
    surface::{Capture, Surface},
};
use anyhow::Result;
use std::time::Instant;
use x11rb::{
    errors::ReplyError,
    protocol::{
        ErrorKind,
        xproto::{ConnectionExt, Window},
    },
};

#[derive(Default)]
pub(crate) struct Scene {
    pub(crate) windows: Vec<Surface>,
}

impl Scene {
    pub(crate) fn add(&mut self, window: Window, context: &Capture<'_>) -> Result<()> {
        if self.windows.iter().any(|s| s.window == window && s.mapped) {
            return Ok(());
        }
        self.windows.retain(|s| s.window != window);
        match Surface::capture(window, context) {
            Ok(Some(surface)) => self.windows.push(surface),
            Ok(None) => (),
            Err(error) if vanished(&error) => {
                tracing::debug!(window, "window disappeared during capture");
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(crate) fn restack(&mut self, session: &Session) -> Result<()> {
        let children = session
            .conn
            .query_tree(session.screen.root)?
            .reply()?
            .children;
        self.windows.sort_by_key(|surface| {
            children
                .iter()
                .position(|id| *id == surface.window)
                .unwrap_or(children.len())
        });
        Ok(())
    }

    pub(crate) fn close(&mut self, window: Window) {
        if let Some(surface) = self.windows.iter_mut().find(|s| s.window == window) {
            surface.close(Instant::now());
        }
    }

    pub(crate) fn animate(&mut self, now: Instant) -> bool {
        self.windows
            .retain(|surface| surface.mapped || surface.fade.active(now));
        self.windows.iter().any(|surface| surface.fade.active(now))
    }
}

pub(crate) fn vanished(error: &anyhow::Error) -> bool {
    matches!(error.downcast_ref::<ReplyError>(), Some(ReplyError::X11Error(error))
        if matches!(error.error_kind, ErrorKind::Window | ErrorKind::Drawable | ErrorKind::Pixmap | ErrorKind::Match))
}
