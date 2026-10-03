use crate::{config::Config, renderer::Renderer, scene::Scene, session::Session, surface::Capture};
use anyhow::Result;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use x11rb::{connection::Connection, protocol::xproto::ConnectionExt as _};

/// Present normally reports a submission within one refresh. Past this wait, the events are
/// treated as lost so one missing notification cannot freeze the screen.
const PRESENT_TIMEOUT: Duration = Duration::from_secs(1);
const IDLE_POLL: Duration = Duration::from_secs(1);

pub(crate) struct Compositor {
    pub(crate) scene: Scene,
    pub(crate) renderer: Renderer,
    pub(crate) config: Config,
    pub(crate) session: Session,
    pub(crate) dirty: bool,
    pub(crate) resizing: bool,
    pub(crate) running: bool,
}

impl Compositor {
    pub(crate) fn new(mut session: Session, config: Config) -> Result<Self> {
        session.acquire()?;
        let renderer = Renderer::new(&session, &config)?;
        let mut scene = Scene::default();
        let context = Capture {
            conn: &session.conn,
            formats: &renderer.formats,
            atoms: &session.atoms,
            config: &config,
        };
        for window in session
            .conn
            .query_tree(session.screen.root)?
            .reply()?
            .children
        {
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
            session,
            dirty: true,
            resizing: false,
            running: true,
        })
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        let shutdown = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&shutdown))?;
        let mut next_frame = Instant::now();
        let mut was_animating = false;
        // A submission timed out and no later one has finished.
        let mut stalled = false;
        while self.running && !shutdown.load(Ordering::Relaxed) {
            let mut budget_exhausted = true;
            for _ in 0..512 {
                let Some(event) = self.session.conn.poll_for_event()? else {
                    budget_exhausted = false;
                    break;
                };
                self.handle(event)?;
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
                if self.resizing {
                    self.renderer = Renderer::new(&self.session, &self.config)?;
                    self.resizing = false;
                }
                self.renderer
                    .paint(&self.session, &self.scene, &self.config)?;
                self.dirty = false;
                next_frame = Instant::now() + self.config.frame_interval();
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
            let timespec = Timespec::try_from(timeout)?;
            let mut fds = [PollFd::new(self.session.conn.stream(), PollFlags::IN)];
            match poll(&mut fds, Some(&timespec)) {
                Ok(_) | Err(rustix::io::Errno::INTR) => (),
                Err(error) => return Err(error.into()),
            }
        }
        tracing::info!("compositor stopped");
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
