mod pixels;
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
    _config: NamedTempFile,
    damage: u32,
}

impl Desktop {
    pub(crate) fn new(settings: &str) -> Result<Self> {
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
        let compositor = Process(
            Command::new(env!("CARGO_BIN_EXE_compust"))
                .args(["--display", &display, "--config"])
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
        Ok(Self {
            conn,
            root,
            overlay,
            display,
            compositor,
            _server: server,
            _config: config,
            damage,
        })
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
