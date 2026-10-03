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
        xproto::{ConnectionExt as _, Window},
    },
};

impl Compositor {
    pub(crate) fn handle(&mut self, event: Event) -> Result<()> {
        match event {
            Event::CreateNotify(event) => self.clients_changed(&[event.parent])?,
            Event::MapNotify(event)
                if event.event == self.session.screen.root
                    && event.window != self.session.owner
                    && event.window != self.session.overlay =>
            {
                self.scene.add(
                    event.window,
                    &Capture {
                        conn: &self.session.conn,
                        formats: &self.renderer.formats,
                        atoms: &self.session.atoms,
                        config: &self.config,
                    },
                )?;
                self.scene.restack(&self.session)?;
                self.dirty = true;
            }
            Event::UnmapNotify(event) => {
                self.scene.close(event.window);
                self.dirty = true;
            }
            Event::DestroyNotify(event) => {
                self.scene.close(event.window);
                self.clients_changed(&[event.event, event.window])?;
                self.dirty = true;
            }
            Event::ReparentNotify(event) => {
                if event.parent == self.session.screen.root {
                    self.scene.add(
                        event.window,
                        &Capture {
                            conn: &self.session.conn,
                            formats: &self.renderer.formats,
                            atoms: &self.session.atoms,
                            config: &self.config,
                        },
                    )?;
                } else {
                    self.scene.close(event.window);
                }
                self.clients_changed(&[event.event, event.parent, event.window])?;
                self.scene.restack(&self.session)?;
                self.dirty = true;
            }
            Event::ConfigureNotify(event) => {
                if event.window == self.session.screen.root {
                    // Keep a replacement already requested by a RandR change in this batch.
                    self.resizing |= event.width != self.renderer.size.width
                        || event.height != self.renderer.size.height;
                } else {
                    self.configure(event.window)?;
                }
                self.scene.restack(&self.session)?;
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
                self.config.vsync = false;
                self.dirty = true;
            }
            Event::Error(error) => bail!("X11 request failed: {error:?}"),
            _ => (),
        }
        Ok(())
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
                Err(error) if vanished(&error) => surface.close(std::time::Instant::now()),
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
                        surface.close(std::time::Instant::now());
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
                Err(error) if vanished(&error) => surface.close(std::time::Instant::now()),
                Err(error) => return Err(error),
            }
            self.dirty = true;
        }
        Ok(())
    }

    fn configure(&mut self, window: Window) -> Result<()> {
        let Some(surface) = self
            .scene
            .windows
            .iter_mut()
            .find(|s| s.window == window && s.mapped)
        else {
            return Ok(());
        };
        let result = (|| -> Result<()> {
            let geometry = self.session.conn.get_geometry(window)?.reply()?;
            if geometry.width != surface.geometry.width
                || geometry.height != surface.geometry.height
                || geometry.border_width != surface.geometry.border_width
            {
                if let Some(mut replacement) = Surface::capture(
                    window,
                    &Capture {
                        conn: &self.session.conn,
                        formats: &self.renderer.formats,
                        atoms: &self.session.atoms,
                        config: &self.config,
                    },
                )? {
                    std::mem::swap(&mut replacement.fade, &mut surface.fade);
                    *surface = replacement;
                }
            } else {
                surface.geometry = geometry;
                surface.refresh_shape()?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => (),
            Err(error) if vanished(&error) => surface.close(std::time::Instant::now()),
            Err(error) => return Err(error),
        }
        Ok(())
    }
}
