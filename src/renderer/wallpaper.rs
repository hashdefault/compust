use super::Renderer;
use crate::{picture::Picture, session::Session};
use anyhow::{Context, Result};
use x11rb::{
    NONE,
    protocol::{render::Repeat, xproto::ConnectionExt as _},
};
impl Renderer {
    pub(crate) fn refresh_wallpaper(&mut self, session: &Session) -> Result<()> {
        self.wallpaper = None;
        self.wallpaper_pixmap = None;
        // A setter may draw into the same pixmap again, so the GPU painter imports it anew.
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.forget_wallpaper();
        }
        for atom in session.atoms.wallpaper {
            let reply = session
                .conn
                .get_property(
                    false,
                    session.screen.root,
                    atom,
                    x11rb::protocol::xproto::AtomEnum::PIXMAP,
                    0,
                    1,
                )?
                .reply()?;
            if let Some(pixmap) = reply
                .value32()
                .and_then(|mut values| values.next())
                .filter(|id| *id != NONE)
            {
                let format = self
                    .formats
                    .screens
                    .get(session.screen_number)
                    .context("missing screen")?
                    .depths
                    .iter()
                    .flat_map(|d| &d.visuals)
                    .find(|v| v.visual == session.screen.root_visual)
                    .context("missing visual")?
                    .format;
                match Picture::borrowed(&session.conn, pixmap, format) {
                    Ok(picture) => {
                        picture.repeat(Repeat::NORMAL)?;
                        self.wallpaper = Some(picture);
                        self.wallpaper_pixmap = Some(pixmap);
                        break;
                    }
                    Err(error) if crate::scene::vanished(&error) => {
                        tracing::debug!(pixmap, "wallpaper pixmap is stale");
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }
}
