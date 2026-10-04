use crate::{
    atoms::active_window,
    compositor::{Compositor, Output},
    region::Rect,
    renderer::Source,
    scene::vanished,
    surface::{Capture, Surface},
};
use anyhow::{Result, bail};
use x11rb::{
    NONE,
    protocol::{
        Event,
        damage::ConnectionExt as _,
        xproto::{ConfigureNotifyEvent, ConnectionExt as _, MapState, Place, Window, WindowClass},
    },
};

impl Compositor {
    /// Handle one event; `sequence` orders it against the last window tree query.
    pub(crate) fn handle(&mut self, event: Event, sequence: u64) -> Result<()> {
        if self.output == Output::Direct && self.wakes(&event)? {
            self.resume()?;
        }
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
            // Repainting for a window that shows nothing would hold the next real frame back
            // by a vblank, because the single buffer waits for each submission to finish.
            Event::UnmapNotify(event) => {
                self.dirty |= self.scene.close(event.window, &self.config);
            }
            Event::DestroyNotify(event) => {
                self.dirty |= self.scene.close(event.window, &self.config);
                self.clients_changed(&[event.event, event.window])?;
            }
            Event::ReparentNotify(event) => {
                if event.parent == root {
                    self.add(event.window)?;
                } else {
                    self.scene.close(event.window, &self.config);
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
                    self.dirty = true;
                } else {
                    let changed = self.configure(&event)?;
                    self.dirty |= self.scene.restack(&self.session)? || changed;
                }
            }
            Event::CirculateNotify(_) => {
                self.dirty |= self.scene.restack(&self.session)?;
            }
            Event::DamageNotify(event) => {
                if let Some(surface) = self.scene.windows.iter().find(|s| s.damage == event.damage)
                {
                    self.session
                        .conn
                        .damage_subtract(event.damage, NONE, NONE)?;
                    self.renderer
                        .damage(Source::Window(surface.window), surface.damaged(event.area));
                    self.dirty = true;
                }
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
            Event::Expose(event) => {
                self.renderer.damage(
                    Source::Output,
                    Rect::new(
                        i32::from(event.x),
                        i32::from(event.y),
                        event.width,
                        event.height,
                    ),
                );
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

    /// Whether handling `event` needs compositing, because it captures a window or replaces
    /// the renderer: a window that maps, arrives on the root, or changes size, and a change
    /// of the screen. Any other event is handled as it is, and compositing resumes
    /// afterwards if the scene then asks for it.
    fn wakes(&self, event: &Event) -> Result<bool> {
        let root = self.session.screen.root;
        Ok(match event {
            Event::MapNotify(event) if event.event == root => {
                event.window != self.session.owner
                    && event.window != self.session.overlay
                    && self.viewable(event.window)?
            }
            Event::ReparentNotify(event) => event.parent == root,
            Event::ConfigureNotify(event) if event.window == root => true,
            Event::ConfigureNotify(event) => self.scene.windows.iter().any(|surface| {
                let captured = &surface.geometry;
                surface.window == event.window
                    && surface.mapped
                    && (captured.width, captured.height, captured.border_width)
                        != (event.width, event.height, event.border_width)
            }),
            Event::RandrScreenChangeNotify(_) => true,
            _ => false,
        })
    }

    /// Whether `window` shows anything: it still exists, is mapped, and is not input-only.
    fn viewable(&self, window: Window) -> Result<bool> {
        let attributes = self.session.conn.get_window_attributes(window)?.reply();
        match attributes.map_err(anyhow::Error::from) {
            Ok(attributes) => Ok(attributes.class != WindowClass::INPUT_ONLY
                && attributes.map_state == MapState::VIEWABLE),
            Err(error) if vanished(&error) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn add(&mut self, window: Window) -> Result<()> {
        self.scene.add(
            window,
            &Capture {
                conn: &self.session.conn,
                formats: &self.renderer.formats,
                atoms: &self.session.atoms,
                config: &self.config,
                active: self.scene.active,
                replaces: None,
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
                    surface.close(std::time::Instant::now(), &self.config);
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
            self.renderer.invalidate();
            self.dirty = true;
        }
        if event.window == self.session.screen.root
            && event.atom == self.session.atoms.active_window
        {
            // Rules that choose by focus now give other settings to the windows that lost
            // and gained it.
            let active = active_window(&self.session.conn, event.window, event.atom)?;
            self.scene.active = active;
            for surface in self.scene.windows.iter_mut().filter(|s| s.mapped) {
                self.dirty |= surface.focus(active, &self.config);
            }
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
                        surface.close(std::time::Instant::now(), &self.config);
                    }
                    Err(error) => return Err(error),
                }
            }
            self.dirty = true;
        } else if self.session.atoms.identifies(event.atom) {
            for surface in self
                .scene
                .windows
                .iter_mut()
                .filter(|s| s.mapped && s.identified_by(event.window))
            {
                self.dirty |= surface.refresh_identity(&self.session.atoms, &self.config)?;
            }
        }
        Ok(())
    }

    fn clients_changed(&mut self, windows: &[Window]) -> Result<()> {
        let active = self.scene.active;
        for surface in self
            .scene
            .windows
            .iter_mut()
            .filter(|s| s.mapped && windows.iter().any(|window| s.watches(*window)))
        {
            match surface.refresh_client(&self.session.atoms, &self.config, active) {
                Ok(()) => (),
                Err(error) if vanished(&error) => {
                    surface.close(std::time::Instant::now(), &self.config);
                }
                Err(error) => return Err(error),
            }
            self.dirty = true;
        }
        Ok(())
    }

    /// Follow a mapped window's move or resize; reports whether its surface changed.
    fn configure(&mut self, event: &ConfigureNotifyEvent) -> Result<bool> {
        let window = event.window;
        let active = self.scene.active;
        let Some(surface) = self
            .scene
            .windows
            .iter_mut()
            .find(|s| s.window == window && s.mapped)
        else {
            return Ok(false);
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
            let moved = (surface.geometry.x, surface.geometry.y) != (event.x, event.y);
            surface.geometry.x = event.x;
            surface.geometry.y = event.y;
            return Ok(moved);
        }
        let capture = Surface::capture(
            window,
            &Capture {
                conn: &self.session.conn,
                formats: &self.renderer.formats,
                atoms: &self.session.atoms,
                config: &self.config,
                active,
                replaces: Some(surface),
            },
        );
        match capture {
            Ok(Some(mut replacement)) => {
                std::mem::swap(&mut replacement.fade, &mut surface.fade);
                *surface = replacement;
            }
            Ok(None) => (),
            Err(error) if vanished(&error) => {
                surface.close(std::time::Instant::now(), &self.config);
            }
            Err(error) => return Err(error),
        }
        Ok(true)
    }
}
