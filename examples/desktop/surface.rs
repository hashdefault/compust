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
        present::{ConnectionExt as _, EventMask as PresentMask},
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConnectionExt as _,
            CreateWindowAux, EventMask, WindowClass,
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
}

impl Surface {
    pub(super) fn connect(display: &str) -> Result<Self> {
        let (conn, screen) = x11rb::connect(Some(display))?;
        let screen = conn.setup().roots.get(screen).context("missing screen")?;
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
        conn.present_query_version(1, 2)?.reply()?;
        let selection = conn.intern_atom(false, b"_NET_WM_CM_S0")?.reply()?.atom;
        let deadline = Instant::now() + Duration::from_secs(5);
        while conn.get_selection_owner(selection)?.reply()?.owner == NONE {
            ensure!(
                Instant::now() < deadline,
                "compositor did not own its selection"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let overlay = conn
            .composite_get_overlay_window(root)?
            .reply()?
            .overlay_win;
        let present = conn.generate_id()?;
        let surface = Self {
            conn,
            root,
            overlay,
            present,
            width,
            height,
        };
        ensure!(
            surface
                .property(root, "_NET_SUPPORTING_WM_CHECK")?
                .is_some(),
            "an EWMH window manager is required"
        );
        surface.subscribe()?;
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
                    .override_redirect(u32::from(popup)),
            )?
            .check()?;
        self.conn
            .change_property8(
                x11rb::protocol::xproto::PropMode::REPLACE,
                window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                b"Compust desktop probe",
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
        while !ready()? {
            while let Some(event) = self.conn.poll_for_event()? {
                if let Event::Error(error) = event {
                    anyhow::bail!("probe X11 error during {description}: {error:?}");
                }
            }
            ensure!(Instant::now() < deadline, "timed out: {description}");
            self.wait(deadline.min(Instant::now() + Duration::from_millis(20)))?;
        }
        Ok(())
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
