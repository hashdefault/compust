use anyhow::{Context, Result, ensure};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use x11rb::{
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, NONE,
    connection::Connection,
    protocol::{
        Event,
        composite::ConnectionExt as _,
        damage::{ConnectionExt as _, ReportLevel},
        present::{ConnectionExt as _, EventMask as PresentMask},
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux,
            ConnectionExt as _, CreateWindowAux, EventMask, PropMode, WindowClass,
        },
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

#[path = "pixels.rs"]
mod pixels;

pub(super) struct Surface {
    pub(super) conn: RustConnection,
    pub(super) root: u32,
    pub(super) overlay: u32,
    pub(super) present: u32,
    pub(super) width: u16,
    pub(super) height: u16,
    damage: u32,
}

impl Surface {
    pub(super) fn connect(display: &str) -> Result<Self> {
        let (conn, screen_number) = x11rb::connect(Some(display))?;
        let screen = conn
            .setup()
            .roots
            .get(screen_number)
            .context("missing screen")?;
        ensure!(
            screen.root_depth == 24,
            "probe requires a 24-bit RGB screen"
        );
        let visual = screen
            .allowed_depths
            .iter()
            .flat_map(|depth| &depth.visuals)
            .find(|visual| visual.visual_id == screen.root_visual)
            .context("missing root visual")?;
        ensure!(
            visual.red_mask == 0x00ff_0000
                && visual.green_mask == 0x0000_ff00
                && visual.blue_mask == 0xff,
            "probe requires RGB channel masks"
        );
        ensure!(
            conn.setup()
                .pixmap_formats
                .iter()
                .any(|format| format.depth == 24 && format.bits_per_pixel == 32),
            "probe requires 32-bit pixel storage"
        );
        let (root, width, height) = (screen.root, screen.width_in_pixels, screen.height_in_pixels);
        conn.composite_query_version(0, 4)?.reply()?;
        conn.damage_query_version(1, 1)?.reply()?;
        conn.present_query_version(1, 2)?.reply()?;
        conn.change_window_attributes(
            root,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::STRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE),
        )?
        .check()?;
        let selection = conn
            .intern_atom(false, format!("_NET_WM_CM_S{screen_number}").as_bytes())?
            .reply()?
            .atom;
        let overlay = conn
            .composite_get_overlay_window(root)?
            .reply()?
            .overlay_win;
        let present = conn.generate_id()?;
        let damage = conn.generate_id()?;
        conn.damage_create(damage, overlay, ReportLevel::NON_EMPTY)?
            .check()?;
        let surface = Self {
            conn,
            root,
            overlay,
            present,
            width,
            height,
            damage,
        };
        surface.subscribe()?;
        surface.until("compositor selection", || {
            Ok(surface.conn.get_selection_owner(selection)?.reply()?.owner != NONE)
        })?;
        surface.until("EWMH window manager", || {
            Ok(surface
                .property(root, "_NET_SUPPORTING_WM_CHECK")?
                .is_some())
        })?;
        Ok(surface)
    }

    pub(super) fn subscribe(&self) -> Result<()> {
        self.conn
            .present_select_input(self.present, self.overlay, PresentMask::COMPLETE_NOTIFY)?
            .check()?;
        Ok(())
    }

    pub(super) fn window(&self, color: u32, popup: bool) -> Result<u32> {
        let window = self.conn.generate_id()?;
        self.conn
            .create_window(
                COPY_DEPTH_FROM_PARENT,
                window,
                self.root,
                40,
                40,
                320,
                240,
                0,
                WindowClass::INPUT_OUTPUT,
                COPY_FROM_PARENT,
                &CreateWindowAux::new()
                    .background_pixel(color)
                    .event_mask(EventMask::STRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE)
                    .override_redirect(u32::from(popup)),
            )?
            .check()?;
        self.conn
            .change_property8(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                b"Compust desktop probe",
            )?
            .check()?;
        self.conn.map_window(window)?.check()?;
        Ok(window)
    }

    /// A small window that a window manager can assign to a second workspace by its title.
    pub(super) fn anchor(&self) -> Result<u32> {
        let window = self.conn.generate_id()?;
        self.conn
            .create_window(
                COPY_DEPTH_FROM_PARENT,
                window,
                self.root,
                600,
                500,
                48,
                48,
                0,
                WindowClass::INPUT_OUTPUT,
                COPY_FROM_PARENT,
                &CreateWindowAux::new().background_pixel(0x0080_8080),
            )?
            .check()?;
        self.conn
            .change_property8(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                b"Compust workspace anchor",
            )?
            .check()?;
        self.conn.map_window(window)?.check()?;
        Ok(window)
    }

    pub(super) fn paint(&self, window: u32, color: u32) -> Result<()> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().background_pixel(color),
        )?;
        self.conn.clear_area(false, window, 0, 0, 0, 0)?;
        self.conn.flush()?;
        Ok(())
    }

    pub(super) fn atom(&self, name: &str) -> Result<u32> {
        Ok(self.conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
    }

    pub(super) fn property(&self, window: u32, name: &str) -> Result<Option<u32>> {
        Ok(self
            .conn
            .get_property(false, window, self.atom(name)?, AtomEnum::ANY, 0, 1)?
            .reply()?
            .value32()
            .and_then(|mut values| values.next()))
    }

    /// Up to sixteen 32-bit values of a property, empty when it is absent.
    pub(super) fn properties(&self, window: u32, name: &str) -> Result<Vec<u32>> {
        Ok(self
            .conn
            .get_property(false, window, self.atom(name)?, AtomEnum::ANY, 0, 16)?
            .reply()?
            .value32()
            .map(Iterator::collect)
            .unwrap_or_default())
    }

    /// Root coordinates of the top-left corner of `window`, inside its border.
    pub(super) fn origin(&self, window: u32) -> Result<(i16, i16)> {
        let position = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)?
            .reply()?;
        Ok((position.dst_x, position.dst_y))
    }

    /// Root coordinates of the center of `window`.
    pub(super) fn center(&self, window: u32) -> Result<(i16, i16)> {
        let geometry = self.conn.get_geometry(window)?.reply()?;
        let origin = self.origin(window)?;
        Ok((
            origin.0 + i16::try_from(geometry.width / 2)?,
            origin.1 + i16::try_from(geometry.height / 2)?,
        ))
    }

    /// The root's child containing `window`: its frame under a reparenting window manager.
    pub(super) fn frame(&self, window: u32) -> Result<u32> {
        let mut frame = window;
        loop {
            let parent = self.conn.query_tree(frame)?.reply()?.parent;
            if parent == self.root || parent == NONE {
                return Ok(frame);
            }
            frame = parent;
        }
    }

    /// Send the window manager a root property event. The probe receives it too, so a wait
    /// that nudges is evaluated again immediately.
    pub(super) fn nudge(&self) -> Result<()> {
        self.conn
            .change_property8(
                PropMode::REPLACE,
                self.root,
                self.atom("_COMPUST_PROBE_NUDGE")?,
                AtomEnum::STRING,
                b"",
            )?
            .check()?;
        Ok(())
    }

    /// Remove the nudge property and discard the events the nudges produced.
    pub(super) fn settle(&self) -> Result<()> {
        self.conn
            .delete_property(self.root, self.atom("_COMPUST_PROBE_NUDGE")?)?
            .check()?;
        self.conn.sync()?;
        while self.poll_event()?.is_some() {}
        Ok(())
    }

    /// Ask the window manager to move `window`, as an application does.
    pub(super) fn place(&self, window: u32, position: (i32, i32)) -> Result<()> {
        self.conn
            .configure_window(
                window,
                &ConfigureWindowAux::new().x(position.0).y(position.1),
            )?
            .check()?;
        Ok(())
    }

    pub(super) fn message(&self, window: u32, name: &str, data: [u32; 5]) -> Result<()> {
        let event = ClientMessageEvent::new(32, window, self.atom(name)?, data);
        self.conn
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )?
            .check()?;
        Ok(())
    }

    pub(super) fn until(
        &self,
        description: &str,
        mut ready: impl FnMut() -> Result<bool>,
    ) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            ensure!(Instant::now() < deadline, "timed out: {description}");
            self.conn.damage_subtract(self.damage, NONE, NONE)?;
            self.conn.flush()?;
            if ready()? {
                return Ok(());
            }
            while self.poll_event()?.is_none() {
                ensure!(Instant::now() < deadline, "timed out: {description}");
                self.wait(deadline)?;
            }
        }
    }

    pub(super) fn poll_event(&self) -> Result<Option<Event>> {
        let event = self.conn.poll_for_event()?;
        match &event {
            Some(Event::Error(error)) => anyhow::bail!("probe X11 error: {error:?}"),
            Some(Event::DamageNotify(event)) if event.damage == self.damage => {
                self.conn.damage_subtract(self.damage, NONE, NONE)?;
                self.conn.flush()?;
            }
            _ => {}
        }
        Ok(event)
    }

    pub(super) fn wait(&self, deadline: Instant) -> Result<()> {
        let duration = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(duration)?;
        poll(
            &mut [PollFd::new(self.conn.stream(), PollFlags::IN)],
            Some(&timeout),
        )?;
        Ok(())
    }

    pub(super) fn screenshot(&self, path: &Path) -> Result<()> {
        pixels::screenshot(self, path)
    }
}
