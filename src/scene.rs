use crate::{
    session::Session,
    surface::{Capture, Surface},
};
use anyhow::Result;
use std::time::{Duration, Instant};
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
        match Surface::capture(window, context) {
            Ok(Some(mut surface)) => {
                if let Some(previous) = self.windows.iter_mut().find(|s| s.window == window) {
                    previous
                        .fade
                        .reopen(Instant::now(), context.config.fade_duration());
                    std::mem::swap(&mut surface.fade, &mut previous.fade);
                    *previous = surface;
                } else {
                    self.windows.push(surface);
                }
            }
            Ok(None) => (),
            Err(error) if vanished(&error) => {
                tracing::debug!(window, "window disappeared during capture");
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub(crate) fn restack(&mut self, session: &Session) -> Result<()> {
        let mut children = session
            .conn
            .query_tree(session.screen.root)?
            .reply()?
            .children;
        // Keep fading snapshots below their former upper neighbor after destruction.
        for (index, surface) in self.windows.iter().enumerate().rev() {
            if !children.contains(&surface.window) {
                let above = self
                    .windows
                    .iter()
                    .skip(index + 1)
                    .find_map(|upper| children.iter().position(|id| *id == upper.window));
                children.insert(above.unwrap_or(children.len()), surface.window);
            }
        }
        self.windows.sort_by_key(|surface| {
            children
                .iter()
                .position(|id| *id == surface.window)
                .unwrap_or(children.len())
        });
        Ok(())
    }

    pub(crate) fn close(&mut self, window: Window, fade: Duration) {
        if let Some(surface) = self.windows.iter_mut().find(|s| s.window == window) {
            surface.close(Instant::now(), fade);
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
