use crate::{
    compositor::Compositor,
    scene::vanished,
    surface::{Capture, Surface},
};
use anyhow::{Result, bail};
use x11rb::{
    NONE,
    protocol::{
        Event,
        damage::ConnectionExt as _,
        xproto::{ConfigureNotifyEvent, Place, Window},
    },
};

impl Compositor {
    /// Handle one event; `sequence` orders it against the last window tree query.
    pub(crate) fn handle(&mut self, event: Event, sequence: u64) -> Result<()> {
        self.track_stacking(&event, sequence);
        let root = self.session.screen.root;
        match event {
            Event::CreateNotify(event) => self.clients_changed(&[event.parent])?,
            Event::MapNotify(event)
                if event.event == root
                    && event.window != self.session.owner
                    && event.window != self.session.overlay =>
            {
                self.add(event.window)?;
                self.scene.restack(&self.session)?;
                self.dirty = true;
            }
            Event::UnmapNotify(event) => {
                self.scene.close(event.window, self.config.fade_duration());
                self.dirty = true;
            }
            Event::DestroyNotify(event) => {
                self.scene.close(event.window, self.config.fade_duration());
                self.clients_changed(&[event.event, event.window])?;
                self.dirty = true;
            }
            Event::ReparentNotify(event) => {
                if event.parent == root {
                    self.add(event.window)?;
                } else {
                    self.scene.close(event.window, self.config.fade_duration());
                }
                self.clients_changed(&[event.event, event.parent, event.window])?;
                self.scene.restack(&self.session)?;
                self.dirty = true;
            }
            Event::ConfigureNotify(event) => {
                if event.window == root {
                    // Keep a replacement already requested by a RandR change in this batch.
                    self.resizing |= event.width != self.renderer.size.width
                        || event.height != self.renderer.size.height;
                } else {
                    self.configure(&event)?;
                    self.scene.restack(&self.session)?;
                }
                self.dirty = true;
            }
            Event::CirculateNotify(_) => {
                self.scene.restack(&self.session)?;
                self.dirty = true;
            }
            Event::DamageNotify(event)
                if self.scene.windows.iter().any(|s| s.damage == event.damage) =>
            {
                self.session
                    .conn
                    .damage_subtract(event.damage, NONE, NONE)?;
                self.dirty = true;
            }
            Event::ShapeNotify(event) => self.shape_changed(event.affected_window)?,
            Event::PropertyNotify(event) => self.property_changed(event)?,
            Event::PresentIdleNotify(event)
                if event.serial == self.renderer.serial
                    && Some(event.event) == self.renderer.event_id =>
            {
                self.renderer.idle = true;
            }
            Event::PresentCompleteNotify(event)
                if event.serial == self.renderer.serial
                    && Some(event.event) == self.renderer.event_id =>
            {
                self.renderer.complete = true;
            }
            Event::RandrScreenChangeNotify(_) => {
                self.resizing = true;
                self.dirty = true;
            }
            Event::Expose(_) => {
                self.dirty = true;
            }
            Event::SelectionClear(event) if event.selection == self.session.atoms.selection => {
                self.running = false;
            }
            Event::Error(error) if self.renderer.recover_present(&error)? => {
                // Later renderers, after a root resize or a reload, must not use Present again.
                self.session.capabilities.present = false;
                self.dirty = true;
            }
            Event::Error(error) => bail!("X11 request failed: {error:?}"),
            _ => (),
        }
        Ok(())
    }

    fn add(&mut self, window: Window) -> Result<()> {
        self.scene.add(
            window,
            &Capture {
                conn: &self.session.conn,
                formats: &self.renderer.formats,
                atoms: &self.session.atoms,
                config: &self.config,
            },
        )
    }

    /// Mirror the root's stacking order from a structure event; see `Stack`.
    fn track_stacking(&mut self, event: &Event, sequence: u64) {
        let root = self.session.screen.root;
        let stack = &mut self.scene.stack;
        match event {
            Event::CreateNotify(event) if event.parent == root => {
                stack.raise(sequence, event.window);
            }
            Event::DestroyNotify(event) if event.event == root => {
                stack.remove(sequence, event.window);
            }
            Event::ReparentNotify(event) if event.event == root && event.parent == root => {
                stack.raise(sequence, event.window);
            }
            Event::ReparentNotify(event) if event.event == root => {
                stack.remove(sequence, event.window);
            }
            Event::ConfigureNotify(event) if event.event == root && event.window != root => {
                stack.place(sequence, event.window, event.above_sibling);
            }
            Event::CirculateNotify(event)
                if event.event == root && event.place == Place::ON_TOP =>
            {
                stack.raise(sequence, event.window);
            }
            Event::CirculateNotify(event) if event.event == root => {
                stack.lower(sequence, event.window);
            }
            _ => (),
        }
    }

    fn shape_changed(&mut self, window: Window) -> Result<()> {
        if let Some(surface) = self
            .scene
            .windows
            .iter_mut()
            .find(|s| s.window == window && s.mapped)
        {
            match surface.refresh_shape() {
                Ok(()) => (),
                Err(error) if vanished(&error) => {
                    surface.close(std::time::Instant::now(), self.config.fade_duration());
                }
                Err(error) => return Err(error),
            }
            self.dirty = true;
        }
        Ok(())
    }

    fn property_changed(
        &mut self,
        event: x11rb::protocol::xproto::PropertyNotifyEvent,
    ) -> Result<()> {
        if event.window == self.session.screen.root
            && self.session.atoms.wallpaper.contains(&event.atom)
        {
            self.renderer.refresh_wallpaper(&self.session)?;
            self.dirty = true;
        }
        if event.atom == self.session.atoms.wm_state {
            self.clients_changed(&[event.window])?;
        } else if event.atom == self.session.atoms.opacity {
            for surface in self
                .scene
                .windows
                .iter_mut()
                .filter(|s| s.mapped && s.watches(event.window))
            {
                match surface.refresh_opacity(&self.session.atoms) {
                    Ok(()) => (),
                    Err(error) if vanished(&error) => {
                        surface.close(std::time::Instant::now(), self.config.fade_duration());
                    }
                    Err(error) => return Err(error),
                }
            }
            self.dirty = true;
        }
        Ok(())
    }

    fn clients_changed(&mut self, windows: &[Window]) -> Result<()> {
        for surface in self
            .scene
            .windows
            .iter_mut()
            .filter(|s| s.mapped && windows.iter().any(|window| s.watches(*window)))
        {
            match surface.refresh_client(&self.session.atoms) {
                Ok(()) => (),
                Err(error) if vanished(&error) => {
                    surface.close(std::time::Instant::now(), self.config.fade_duration());
                }
                Err(error) => return Err(error),
            }
            self.dirty = true;
        }
        Ok(())
    }

    fn configure(&mut self, event: &ConfigureNotifyEvent) -> Result<()> {
        let window = event.window;
        let Some(surface) = self
            .scene
            .windows
            .iter_mut()
            .find(|s| s.window == window && s.mapped)
        else {
            return Ok(());
        };
        let captured = &surface.geometry;
        // Each resize gives the window a new pixmap and is reported, so a size in any event
        // that differs from the captured one means the pixmap is stale, including for a window
        // resized and restored before this event is handled. A capture reads the current state.
        if event.width == captured.width
            && event.height == captured.height
            && event.border_width == captured.border_width
        {
            // A move keeps the pixmap and its shape; later events carry later positions.
            surface.geometry.x = event.x;
            surface.geometry.y = event.y;
            return Ok(());
        }
        let capture = Surface::capture(
            window,
            &Capture {
                conn: &self.session.conn,
                formats: &self.renderer.formats,
                atoms: &self.session.atoms,
                config: &self.config,
            },
        );
        match capture {
            Ok(Some(mut replacement)) => {
                std::mem::swap(&mut replacement.fade, &mut surface.fade);
                *surface = replacement;
            }
            Ok(None) => (),
            Err(error) if vanished(&error) => {
                surface.close(std::time::Instant::now(), self.config.fade_duration());
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }
}
