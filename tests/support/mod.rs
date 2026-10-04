pub(crate) mod capture;
pub(crate) mod capture_proxy;
pub(crate) mod monitors;
mod pixels;
pub(crate) mod presentation;
pub(crate) mod proxy;
pub(crate) mod requests;
pub(crate) mod resources;
use anyhow::{Context, Result, ensure};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        Event,
        composite::ConnectionExt as _,
        damage::{ConnectionExt as _, ReportLevel},
        present::{CompleteKind, ConnectionExt as _, EventMask as PresentMask},
        xproto::*,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

pub(crate) struct Process(pub(crate) Child);
impl Drop for Process {
    fn drop(&mut self) {
        if let Err(error) = self.0.kill() {
            eprintln!("test process cleanup: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("test process reap: {error}");
        }
    }
}

pub(crate) struct Desktop {
    pub(crate) conn: RustConnection,
    pub(crate) root: Window,
    pub(crate) overlay: Window,
    pub(crate) display: String,
    pub(crate) compositor: Process,
    _server: Process,
    config: NamedTempFile,
    damage: u32,
}

impl Desktop {
    pub(crate) fn new(settings: &str) -> Result<Self> {
        Self::with_display(settings, |display| Ok((display.to_owned(), ())))
            .map(|(desktop, ())| desktop)
    }

    pub(crate) fn with_display<T>(
        settings: &str,
        route: impl FnOnce(&str) -> Result<(String, T)>,
    ) -> Result<(Self, T)> {
        let executable = std::env::var_os("XVFB").unwrap_or_else(|| "Xvfb".into());
        let mut server = Process(
            Command::new(executable)
                .args([
                    "-displayfd",
                    "1",
                    "-screen",
                    "0",
                    "320x240x24",
                    "-nolisten",
                    "tcp",
                    "-noreset",
                    "-ac",
                    "-nocursor",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .context("starting Xvfb (install it or set XVFB)")?,
        );
        let stdout = server.0.stdout.take().context("missing Xvfb stdout")?;
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            if let Err(error) = sender.send(result) {
                eprintln!("Xvfb readiness channel: {error}");
            }
        });
        let number = receiver.recv_timeout(Duration::from_secs(10))??;
        ensure!(
            !number.trim().is_empty(),
            "Xvfb exited without a display number"
        );
        let display = format!(":{}", number.trim());
        let (conn, screen) = x11rb::connect(Some(&display))?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .context("missing screen")?
            .root;
        conn.change_window_attributes(
            root,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::STRUCTURE_NOTIFY),
        )?
        .check()?;
        conn.composite_query_version(0, 4)?.reply()?;
        conn.damage_query_version(1, 1)?.reply()?;
        let mut config = NamedTempFile::new()?;
        write!(config, "{settings}")?;
        let (compositor_display, transport) = route(&display)?;
        let compositor = Process(
            Command::new(env!("CARGO_BIN_EXE_compust"))
                .args(["--display", &compositor_display, "--config"])
                .arg(config.path())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()?,
        );
        let selection = conn.intern_atom(false, b"_NET_WM_CM_S0")?.reply()?.atom;
        let deadline = Instant::now() + Duration::from_secs(10);
        while conn.get_selection_owner(selection)?.reply()?.owner == NONE {
            next_event(&conn, deadline)?;
        }
        let overlay = conn
            .composite_get_overlay_window(root)?
            .reply()?
            .overlay_win;
        let damage = conn.generate_id()?;
        conn.damage_create(damage, overlay, ReportLevel::NON_EMPTY)?
            .check()?;
        Ok((
            Self {
                conn,
                root,
                overlay,
                display,
                compositor,
                _server: server,
                config,
                damage,
            },
            transport,
        ))
    }

    /// Replaces the configuration file and asks the running compositor to reload it.
    pub(crate) fn reload(&self, settings: &str) -> Result<()> {
        std::fs::write(self.config.path(), settings)?;
        self.signal_reload()
    }

    /// Removes the configuration file, then asks the compositor to reload it.
    pub(crate) fn reload_missing(&self) -> Result<()> {
        std::fs::remove_file(self.config.path())?;
        self.signal_reload()
    }

    fn signal_reload(&self) -> Result<()> {
        let status = Command::new("kill")
            .args(["-USR1", &self.compositor.0.id().to_string()])
            .status()?;
        ensure!(status.success(), "could not signal the compositor");
        Ok(())
    }

    pub(crate) fn window(&self, rect: Rectangle, color: u32) -> Result<Window> {
        let window = self.conn.generate_id()?;
        self.conn
            .create_window(
                24,
                window,
                self.root,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new().background_pixel(color),
            )?
            .check()?;
        Ok(window)
    }

    pub(crate) fn map(&self, window: Window) -> Result<()> {
        self.conn.map_window(window)?.check()?;
        self.conn.flush()?;
        Ok(())
    }

    /// A window with an alpha channel, transparent until something draws into it, as browsers
    /// make menus.
    pub(crate) fn argb_window(&self, rect: Rectangle) -> Result<Window> {
        let visual = self
            .conn
            .setup()
            .roots
            .iter()
            .flat_map(|screen| &screen.allowed_depths)
            .filter(|depth| depth.depth == 32)
            .flat_map(|depth| &depth.visuals)
            .find(|visual| visual.class == VisualClass::TRUE_COLOR)
            .context("the server has no 32-bit visual")?
            .visual_id;
        let colormap = self.conn.generate_id()?;
        self.conn
            .create_colormap(ColormapAlloc::NONE, colormap, self.root, visual)?
            .check()?;
        let window = self.conn.generate_id()?;
        self.conn
            .create_window(
                32,
                window,
                self.root,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                0,
                WindowClass::INPUT_OUTPUT,
                visual,
                &CreateWindowAux::new()
                    .background_pixel(0)
                    .border_pixel(0)
                    .colormap(colormap),
            )?
            .check()?;
        Ok(window)
    }

    /// Draw `area` of `window` in `color`, as a client repainting part of itself.
    pub(crate) fn fill(&self, window: Window, area: Rectangle, color: u32) -> Result<()> {
        let gc = self.conn.generate_id()?;
        self.conn
            .create_gc(gc, window, &CreateGCAux::new().foreground(color))?
            .check()?;
        self.conn
            .poly_fill_rectangle(window, gc, &[area])?
            .check()?;
        self.conn.free_gc(gc)?.check()?;
        Ok(())
    }

    /// Wait until Present reports `count` more vblanks on the overlay's CRTC.
    pub(crate) fn wait_vblanks(&self, count: u64) -> Result<()> {
        let id = self.conn.generate_id()?;
        self.conn
            .present_select_input(id, self.overlay, PresentMask::COMPLETE_NOTIFY)?
            .check()?;
        self.conn
            .present_notify_msc(self.overlay, 1, 0, 0, 0)?
            .check()?;
        let msc = self.notified(id, 1)?;
        self.conn
            .present_notify_msc(self.overlay, 2, msc + count, 0, 0)?
            .check()?;
        self.notified(id, 2)?;
        self.conn
            .present_select_input(id, self.overlay, PresentMask::from(0_u32))?
            .check()?;
        Ok(())
    }

    fn notified(&self, id: u32, serial: u32) -> Result<u64> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Event::PresentCompleteNotify(event) = next_event(&self.conn, deadline)?
                && event.event == id
                && event.serial == serial
                && event.kind == CompleteKind::NOTIFY_MSC
            {
                return Ok(event.msc);
            }
        }
    }

    pub(crate) fn opacity(&self, window: Window, opacity: u32) -> Result<()> {
        let atom = self
            .conn
            .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
            .reply()?
            .atom;
        self.conn
            .change_property32(
                PropMode::REPLACE,
                window,
                atom,
                AtomEnum::CARDINAL,
                &[opacity],
            )?
            .check()?;
        Ok(())
    }
}

pub(crate) fn next_event(conn: &RustConnection, deadline: Instant) -> Result<Event> {
    loop {
        if let Some(event) = conn.poll_for_event()? {
            return Ok(event);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        ensure!(!remaining.is_zero(), "timed out waiting for X11 event");
        let timeout = Timespec::try_from(remaining)?;
        let mut fds = [PollFd::new(conn.stream(), PollFlags::IN)];
        poll(&mut fds, Some(&timeout))?;
    }
}
