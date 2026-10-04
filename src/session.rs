use crate::{atoms::Atoms, capabilities::Capabilities};
use anyhow::{Context, Result, ensure};
use std::rc::Rc;
use x11rb::{
    COPY_DEPTH_FROM_PARENT, NONE,
    connection::Connection,
    protocol::{
        Event,
        composite::{ConnectionExt as _, Redirect},
        randr::{ConnectionExt as _, NotifyMask},
        shape::SK,
        xfixes::ConnectionExt as _,
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConnectionExt,
            CreateWindowAux, EventMask, PropMode, Screen, Window, WindowClass,
        },
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

pub(crate) struct Session {
    pub(crate) conn: Rc<RustConnection>,
    pub(crate) screen: Screen,
    pub(crate) screen_number: usize,
    pub(crate) atoms: Atoms,
    pub(crate) capabilities: Capabilities,
    pub(crate) owner: Window,
    pub(crate) overlay: Window,
}

impl Session {
    pub(crate) fn connect(display: Option<&str>) -> Result<Self> {
        let (conn, screen_number) =
            x11rb::connect(display).context("connecting to X11; check DISPLAY and Xauthority")?;
        let screen = conn
            .setup()
            .roots
            .get(screen_number)
            .context("X11 screen is absent")?
            .clone();
        let atoms = Atoms::new(&conn, screen_number)?;
        let capabilities = Capabilities::query(&conn, screen.root)?;
        Ok(Self {
            conn: Rc::new(conn),
            screen,
            screen_number,
            atoms,
            capabilities,
            owner: NONE,
            overlay: NONE,
        })
    }

    pub(crate) fn acquire(&mut self) -> Result<()> {
        self.capabilities.require_baseline()?;
        let conn = &*self.conn;
        let owner = conn.generate_id()?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            owner,
            self.screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?
        .check()?;
        self.owner = owner;
        conn.change_property8(
            PropMode::REPLACE,
            owner,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"Compust",
        )?;
        conn.change_property8(
            PropMode::REPLACE,
            owner,
            self.atoms.timestamp,
            AtomEnum::STRING,
            b"time",
        )?;
        conn.flush()?;
        let timestamp = loop {
            if let Event::PropertyNotify(event) = conn.wait_for_event()?
                && event.window == owner
                && event.atom == self.atoms.timestamp
            {
                break event.time;
            }
        };
        conn.grab_server()?.check()?;
        let claim = (|| -> Result<()> {
            ensure!(
                conn.get_selection_owner(self.atoms.selection)?
                    .reply()?
                    .owner
                    == NONE,
                "another compositor already owns this screen; stop it before starting Compust"
            );
            conn.composite_redirect_subwindows(self.screen.root, Redirect::MANUAL)?
                .check()
                .context("redirecting windows; another compositor may already be running")?;
            conn.set_selection_owner(owner, self.atoms.selection, timestamp)?
                .check()?;
            ensure!(
                conn.get_selection_owner(self.atoms.selection)?
                    .reply()?
                    .owner
                    == owner,
                "could not acquire compositor selection"
            );
            Ok(())
        })();
        conn.ungrab_server()?.check()?;
        claim?;
        conn.send_event(
            false,
            self.screen.root,
            EventMask::STRUCTURE_NOTIFY,
            ClientMessageEvent::new(
                32,
                self.screen.root,
                self.atoms.manager,
                [timestamp, self.atoms.selection, owner, 0, 0],
            ),
        )?;
        self.prepare_overlay()?;
        self.conn.flush()?;
        Ok(())
    }
    /// Stop redirecting windows, which then draw to the screen directly, and take the overlay
    /// out of their way. The screen shows the same image until a window draws.
    pub(crate) fn suspend(&self) -> Result<()> {
        let conn = &*self.conn;
        conn.composite_unredirect_subwindows(self.screen.root, Redirect::MANUAL)?
            .check()?;
        conn.unmap_window(self.overlay)?.check()?;
        Ok(())
    }

    /// Redirect windows again, each into a new pixmap that starts with what the window shows,
    /// and bring the overlay back above them.
    pub(crate) fn resume(&self) -> Result<()> {
        let conn = &*self.conn;
        conn.composite_redirect_subwindows(self.screen.root, Redirect::MANUAL)?
            .check()
            .context("redirecting windows again; another compositor may have started")?;
        conn.map_window(self.overlay)?.check()?;
        Ok(())
    }

    fn prepare_overlay(&mut self) -> Result<()> {
        let conn = &*self.conn;
        self.overlay = conn
            .composite_get_overlay_window(self.screen.root)?
            .reply()?
            .overlay_win;
        let region = conn.generate_id()?;
        conn.xfixes_create_region(region, &[])?.check()?;
        conn.xfixes_set_window_shape_region(self.overlay, SK::INPUT, 0, 0, region)?
            .check()?;
        conn.xfixes_destroy_region(region)?;
        conn.change_window_attributes(
            self.screen.root,
            &ChangeWindowAttributesAux::new().event_mask(
                EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::STRUCTURE_NOTIFY
                    | EventMask::PROPERTY_CHANGE,
            ),
        )?
        .check()?;
        conn.change_window_attributes(
            self.overlay,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::EXPOSURE),
        )?;
        if self.capabilities.randr {
            conn.randr_select_input(
                self.screen.root,
                NotifyMask::SCREEN_CHANGE | NotifyMask::CRTC_CHANGE | NotifyMask::OUTPUT_CHANGE,
            )?
            .check()?;
        }
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Closing the connection releases selection, redirects and overlay even on startup failure.
        if let Err(error) = self.conn.flush() {
            tracing::debug!(%error, "X11 connection closed during cleanup");
        }
    }
}
