use crate::{
    atoms::active_window,
    config::{Config, Source},
    renderer::Renderer,
    scene::{Scene, vanished},
    session::Session,
    surface::{Capture, Surface},
    watch::Watch,
};
use anyhow::Result;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use signal_hook::consts::{SIGINT, SIGTERM, SIGUSR1};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use x11rb::connection::Connection;

/// Present normally reports a submission within one refresh. Past this wait, the events are
/// treated as lost so one missing notification cannot freeze the screen.
const PRESENT_TIMEOUT: Duration = Duration::from_secs(1);
const IDLE_POLL: Duration = Duration::from_secs(1);

/// Who draws the screen.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Output {
    /// Compust composites the windows into its overlay.
    Composited,
    /// Compositing is suspended: one window hides everything else, and windows draw to the
    /// screen directly.
    Direct,
}

pub(crate) struct Compositor {
    pub(crate) scene: Scene,
    pub(crate) renderer: Renderer,
    pub(crate) config: Config,
    source: Source,
    pub(crate) session: Session,
    pub(crate) dirty: bool,
    pub(crate) resizing: bool,
    pub(crate) output: Output,
    pub(crate) running: bool,
    shutdown: Arc<AtomicBool>,
    reload: Arc<AtomicBool>,
    /// Watches the configuration files, so that saving one reloads it; `None` where the
    /// system cannot, which leaves SIGUSR1.
    watch: Option<Watch>,
}

impl Compositor {
    pub(crate) fn new(mut session: Session, source: Source, config: Config) -> Result<Self> {
        // A client that sees the selection may signal at once, and SIGUSR1 would otherwise
        // end the process, so the handlers come before the claim.
        let shutdown = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, Arc::clone(&shutdown))?;
        signal_hook::flag::register(SIGTERM, Arc::clone(&shutdown))?;
        let reload = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGUSR1, Arc::clone(&reload))?;
        let watch = match Watch::new() {
            Ok(mut watch) => {
                watch.follow(&source.files());
                Some(watch)
            }
            Err(error) => {
                tracing::warn!("saved configuration changes need SIGUSR1 to apply: {error}");
                None
            }
        };
        session.acquire()?;
        let renderer = Renderer::new(&session, &config)?;
        let active = active_window(
            &session.conn,
            session.screen.root,
            session.atoms.active_window,
        )?;
        let mut scene = Scene {
            active,
            ..Scene::default()
        };
        let context = Capture {
            conn: &session.conn,
            formats: &renderer.formats,
            atoms: &session.atoms,
            config: &config,
            active: scene.active,
            replaces: None,
        };
        for window in scene.stack.query(&session)?.to_vec() {
            if window != session.owner && window != session.overlay {
                scene.add(window, &context)?;
            }
        }
        tracing::info!(
            present = renderer.present,
            windows = scene.windows.len(),
            "compositor started"
        );
        Ok(Self {
            scene,
            renderer,
            config,
            source,
            session,
            dirty: true,
            resizing: false,
            output: Output::Composited,
            running: true,
            shutdown,
            reload,
            watch,
        })
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        let mut next_frame = Instant::now();
        let mut was_animating = false;
        // A submission timed out and no later one has finished.
        let mut stalled = false;
        while self.running && !self.shutdown.load(Ordering::Relaxed) {
            let mut budget_exhausted = true;
            for _ in 0..512 {
                let Some((event, sequence)) = self.session.conn.poll_for_event_with_sequence()?
                else {
                    budget_exhausted = false;
                    break;
                };
                self.handle(event, sequence)?;
            }
            let saved = self
                .watch
                .as_mut()
                .is_some_and(|watch| watch.settled(Instant::now()));
            if self.reload.swap(false, Ordering::Relaxed) || saved {
                self.reload_config();
            }
            if self.output == Output::Direct && !self.unredirects()? {
                self.resume()?;
            }
            let now = Instant::now();
            if self.renderer.submitted.is_some() && !self.resizing && self.can_paint() {
                stalled = false;
            }
            if self
                .present_deadline()
                .is_some_and(|deadline| now >= deadline)
            {
                tracing::warn!("Present did not finish a submission; replacing its buffers");
                self.resizing = true;
                // The lost frame may never have been shown. Repaint it once; if Present keeps
                // stalling, wait for new damage instead of repainting every timeout.
                self.dirty |= !stalled;
                stalled = true;
            }
            let previous_count = self.scene.windows.len();
            let animating = self.scene.animate(now);
            self.dirty |= previous_count != self.scene.windows.len() || animating || was_animating;
            was_animating = animating;
            if self.dirty && now >= next_frame && self.can_paint() {
                if self.output == Output::Composited && self.unredirects()? {
                    self.suspend()?;
                }
                // While windows draw to the screen themselves, there is nothing to paint.
                if self.output == Output::Composited {
                    // A reload that changes blur or vsync waits for the buffer like any paint.
                    if self.resizing || !self.renderer.fits(&self.config) {
                        // The server may still show the replaced renderer's last submission.
                        // Continuing its serials tells that submission from the new ones.
                        let serial = self.renderer.serial;
                        self.renderer = Renderer::new(&self.session, &self.config)?;
                        self.renderer.serial = serial;
                        self.resizing = false;
                    }
                    if self
                        .renderer
                        .paint(&self.session, &self.scene, &self.config)?
                    {
                        next_frame = Instant::now() + self.config.frame_interval();
                    }
                }
                self.dirty = false;
            }
            if budget_exhausted {
                continue;
            }
            let timeout = if (self.dirty || animating) && self.can_paint() {
                next_frame.saturating_duration_since(Instant::now())
            } else {
                self.present_deadline().map_or(IDLE_POLL, |deadline| {
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(IDLE_POLL)
                })
            };
            self.wait(timeout)?;
        }
        tracing::info!("compositor stopped");
        Ok(())
    }

    /// Sleep until the X server sends something, a configuration file changes, or `timeout`
    /// passes; sooner when a saved configuration settles first.
    fn wait(&mut self, timeout: Duration) -> Result<()> {
        let timeout = self
            .watch
            .as_ref()
            .and_then(Watch::due)
            .map_or(timeout, |due| {
                timeout.min(due.saturating_duration_since(Instant::now()))
            });
        let timespec = Timespec::try_from(timeout)?;
        let stream = PollFd::new(self.session.conn.stream(), PollFlags::IN);
        let Some(watch) = self.watch.as_mut() else {
            return wait_for(&mut [stream], &timespec);
        };
        let mut fds = [stream, PollFd::new(&*watch, PollFlags::IN)];
        wait_for(&mut fds, &timespec)?;
        let [_, watched] = &fds;
        if watched.revents().contains(PollFlags::IN)
            && let Err(error) = watch.read(Instant::now())
        {
            tracing::warn!("saved configuration changes need SIGUSR1 to apply: {error}");
            self.watch = None;
        }
        Ok(())
    }

    /// Reads the configuration again, as a restart would, and applies its rules to every window.
    /// A file that cannot be read or is invalid leaves the running configuration in place. Fades
    /// in progress keep their duration. The search may now find another file, or a link point
    /// elsewhere, so the files watched are those it would read next.
    fn reload_config(&mut self) {
        if let Some(watch) = self.watch.as_mut() {
            watch.follow(&self.source.files());
        }
        let (config, path) = match self.source.load() {
            Ok(loaded) => loaded,
            Err(error) => {
                tracing::warn!("keeping the current configuration: {error:#}");
                return;
            }
        };
        self.dirty |= config != self.config;
        self.config = config;
        for surface in &mut self.scene.windows {
            surface.apply(&self.config);
        }
        if let Some(path) = path {
            tracing::info!(path = %path.display(), "configuration reloaded");
        } else {
            tracing::info!("no configuration file found; reloaded built-in defaults");
        }
    }

    /// Whether compositing can be suspended: `unredirect_fullscreen` asks for it, and one
    /// surface hides everything else.
    fn unredirects(&self) -> Result<bool> {
        Ok(
            self.config.unredirect_fullscreen
                && self.renderer.covered(&self.scene, &self.config)?,
        )
    }

    /// Stop compositing. Windows draw to the screen directly until `resume`.
    fn suspend(&mut self) -> Result<()> {
        self.session.suspend()?;
        self.output = Output::Direct;
        tracing::debug!("compositing suspended behind a window that covers the screen");
        Ok(())
    }

    /// Composite again. Redirection gives every window a new pixmap, so each surface is
    /// captured again, which repaints all of the screen: the surface that covered it either
    /// changed or left. A surface that closed meanwhile has only its pixmap from before the
    /// suspension, which shows an old image, so it goes without fading.
    pub(crate) fn resume(&mut self) -> Result<()> {
        self.session.resume()?;
        self.output = Output::Composited;
        tracing::debug!("compositing resumed");
        let mut gone = Vec::new();
        let active = self.scene.active;
        for surface in self.scene.windows.iter_mut().filter(|s| s.mapped) {
            let capture = Surface::capture(
                surface.window,
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
                Ok(None) => gone.push(surface.window),
                Err(error) if vanished(&error) => gone.push(surface.window),
                Err(error) => return Err(error),
            }
        }
        self.scene
            .windows
            .retain(|surface| surface.mapped && !gone.contains(&surface.window));
        self.dirty = true;
        Ok(())
    }

    /// Present must release a buffer before it is painted again. A monitor reconfiguration can
    /// discard a pending submission without completion or idle events, and so can a timeout;
    /// the replacement renderer has its own buffers and event selection, so it does not wait.
    fn can_paint(&self) -> bool {
        self.resizing || (self.renderer.idle && self.renderer.complete)
    }

    /// When the outstanding submission is given up, if one is still blocking painting.
    fn present_deadline(&self) -> Option<Instant> {
        if self.can_paint() {
            return None;
        }
        Some(self.renderer.submitted? + PRESENT_TIMEOUT)
    }
}

/// Wait until one of `fds` is ready or `timeout` passes; a signal ends the wait early.
fn wait_for(fds: &mut [PollFd<'_>], timeout: &Timespec) -> Result<()> {
    match poll(fds, Some(timeout)) {
        Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
        Err(error) => Err(error.into()),
    }
}
