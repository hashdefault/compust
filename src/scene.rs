use crate::{
    config::Config,
    session::Session,
    surface::{Capture, Surface},
};
use anyhow::Result;
use std::time::Instant;
use x11rb::{
    NONE,
    errors::ReplyError,
    protocol::{
        ErrorKind,
        xproto::{ConnectionExt, Window},
    },
};

#[derive(Default)]
pub(crate) struct Scene {
    pub(crate) windows: Vec<Surface>,
    pub(crate) stack: Stack,
    /// The window the window manager reports as active; see `atoms::active_window`.
    pub(crate) active: Option<Window>,
}

/// The root's children from bottom to top, mirrored from structure events so that restacking
/// costs no `QueryTree` round trip. A query's reply already reflects every event generated
/// before the server handled it, so those events are skipped by sequence number. An event that
/// names a window the mirror lacks makes it stale, and the next restack queries again.
pub(crate) struct Stack {
    children: Vec<Window>,
    /// Sequence number of the query `children` came from.
    queried: u64,
    stale: bool,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            queried: 0,
            stale: true,
        }
    }
}

impl Stack {
    pub(crate) fn query(&mut self, session: &Session) -> Result<&[Window]> {
        let cookie = session.conn.query_tree(session.screen.root)?;
        self.queried = cookie.sequence_number();
        self.children = cookie.reply()?.children;
        self.stale = false;
        Ok(&self.children)
    }

    /// Apply `change` for an event with `sequence`; it reports whether it knew every window.
    fn update(&mut self, sequence: u64, change: impl FnOnce(&mut Vec<Window>) -> bool) {
        if sequence >= self.queried && !self.stale && !change(&mut self.children) {
            self.stale = true;
        }
    }

    /// A window created on, reparented to, or circulated to the top of the root.
    pub(crate) fn raise(&mut self, sequence: u64, window: Window) {
        self.update(sequence, |children| {
            children.retain(|id| *id != window);
            children.push(window);
            true
        });
    }

    pub(crate) fn lower(&mut self, sequence: u64, window: Window) {
        self.place(sequence, window, NONE);
    }

    /// A window destroyed or reparented away from the root.
    pub(crate) fn remove(&mut self, sequence: u64, window: Window) {
        self.update(sequence, |children| {
            children.retain(|id| *id != window);
            true
        });
    }

    /// Place `window` directly above `sibling`, or at the bottom when `sibling` is `NONE`.
    pub(crate) fn place(&mut self, sequence: u64, window: Window, sibling: Window) {
        self.update(sequence, |children| {
            let Some(from) = children.iter().position(|id| *id == window) else {
                return false;
            };
            children.remove(from);
            let to = if sibling == NONE {
                0
            } else {
                match children.iter().position(|id| *id == sibling) {
                    Some(below) => below + 1,
                    None => return false,
                }
            };
            children.insert(to, window);
            true
        });
    }
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
                        .reopen(Instant::now(), surface.fade_duration(context.config));
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

    /// Sort surfaces into stacking order; reports whether the order changed.
    pub(crate) fn restack(&mut self, session: &Session) -> Result<bool> {
        if self.stack.stale {
            self.stack.query(session)?;
        }
        let mut children = self.stack.children.clone();
        // A closing surface takes its place below the windows that covered it, and above the
        // rest: the neighbor a tiling manager grows over its area goes under it, so that the
        // fade is not cut short by that neighbor.
        let closing: Vec<_> = self
            .windows
            .iter()
            .filter(|surface| surface.blockers.is_some())
            .collect();
        children.retain(|id| !closing.iter().any(|surface| surface.window == *id));
        // Keep other fading snapshots below their former upper neighbor after destruction.
        for (index, surface) in self.windows.iter().enumerate().rev() {
            if surface.blockers.is_none() && !children.contains(&surface.window) {
                let above = self
                    .windows
                    .iter()
                    .skip(index + 1)
                    .find_map(|upper| children.iter().position(|id| *id == upper.window));
                children.insert(above.unwrap_or(children.len()), surface.window);
            }
        }
        // From the lowest up, so that a closing surface covering another lands above it.
        for surface in &closing {
            let covered = surface.blockers.as_deref().unwrap_or_default();
            let below = covered
                .iter()
                .filter_map(|blocker| children.iter().position(|id| id == blocker))
                .min();
            children.insert(below.unwrap_or(children.len()), surface.window);
        }
        let position = |surface: &Surface| {
            children
                .iter()
                .position(|id| *id == surface.window)
                .unwrap_or(children.len())
        };
        if self.windows.is_sorted_by_key(position) {
            return Ok(false);
        }
        self.windows.sort_by_key(position);
        Ok(true)
    }

    /// Start closing the mapped surface of `window`; reports whether there was one.
    pub(crate) fn close(&mut self, window: Window, config: &Config) -> bool {
        let Some(index) = self
            .windows
            .iter()
            .position(|s| s.window == window && s.mapped)
        else {
            return false;
        };
        let area = self.windows.get(index).map(Surface::bounds);
        let blockers = self
            .windows
            .iter()
            .skip(index + 1)
            .filter(|upper| area.is_some_and(|area| upper.bounds().intersect(area).is_some()))
            .map(|upper| upper.window)
            .collect();
        if let Some(surface) = self.windows.get_mut(index) {
            surface.close(Instant::now(), config);
            surface.blockers = Some(blockers);
        }
        true
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mirror(children: &[Window], queried: u64) -> Stack {
        Stack {
            children: children.to_vec(),
            queried,
            stale: false,
        }
    }

    #[test]
    fn events_before_the_query_are_already_in_it() {
        // Given a snapshot from request 10, an event generated before it changes nothing
        // and one generated after it applies.
        let mut stack = mirror(&[1, 2, 3], 10);
        stack.place(9, 1, 3);
        assert_eq!(stack.children, [1, 2, 3]);
        stack.place(10, 1, 3);
        assert_eq!(stack.children, [2, 3, 1]);
        stack.lower(11, 1);
        stack.raise(11, 4);
        stack.remove(11, 2);
        assert_eq!(stack.children, [1, 3, 4]);
        assert!(!stack.stale);
    }

    #[test]
    fn unknown_windows_make_the_mirror_stale() {
        // Given a sibling or window the mirror lacks, it asks for a new query and stops
        // applying events until then.
        let mut stack = mirror(&[1, 2], 10);
        stack.place(11, 1, 9);
        assert!(stack.stale);
        stack.raise(12, 5);
        assert!(!stack.children.contains(&5));
        let mut stack = mirror(&[1, 2], 10);
        stack.place(11, 9, 1);
        assert!(stack.stale);
    }
}
