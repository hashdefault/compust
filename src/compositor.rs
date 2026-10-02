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
            let previous_count = self.scene.windows.len();
            let animating = self.scene.animate(now);
            self.dirty |= previous_count != self.scene.windows.len() || animating || was_animating;
            was_animating = animating;
            if self.dirty && now >= next_frame && self.renderer.idle && self.renderer.complete {
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
            let timeout =
                if (self.dirty || animating) && self.renderer.idle && self.renderer.complete {
                    next_frame.saturating_duration_since(Instant::now())
                } else {
                    Duration::from_secs(1)
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
}
