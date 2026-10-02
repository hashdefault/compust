use super::Renderer;
use crate::session::Session;
use anyhow::{Context, Result};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        present::{ConnectionExt as _, EventMask, Option as PresentOption},
        render::{ConnectionExt as _, PictOp},
        xproto::Rectangle,
    },
};
impl Renderer {
    pub(super) fn submit(&mut self, session: &Session) -> Result<()> {
        let conn = &session.conn;
        let full = Rectangle {
            x: 0,
            y: 0,
            width: self.size.width,
            height: self.size.height,
        };
        if self.present {
            self.serial = self.serial.wrapping_add(1);
            conn.present_pixmap(
                session.overlay,
                self.back.pixmap.context("back buffer lacks pixmap")?,
                self.serial,
                NONE,
                NONE,
                0,
                0,
                NONE,
                NONE,
                NONE,
                u32::from(PresentOption::COPY),
                0,
                0,
                0,
                &[],
            )?;
            self.idle = false;
            self.complete = false;
        } else {
            conn.render_composite(
                PictOp::SRC,
                self.back.id,
                NONE,
                self.output.id,
                0,
                0,
                0,
                0,
                0,
                0,
                full.width,
                full.height,
            )?;
        }
        conn.flush()?;

        Ok(())
    }
}
impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(id) = self.event_id {
            let result = (|| -> Result<()> {
                self.conn
                    .present_select_input(id, self.overlay, EventMask::from(0_u32))?
                    .check()?;
                Ok(())
            })();
            if let Err(error) = result {
                tracing::debug!(%error, "presentation subscription cleanup failed");
            }
        }
    }
}
