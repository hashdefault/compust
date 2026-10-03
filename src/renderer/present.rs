use super::Renderer;
use crate::session::Session;
use anyhow::{Context, Result, ensure};
use x11rb::{
    NONE,
    connection::Connection,
    protocol::{
        ErrorKind,
        present::{ConnectionExt as _, EventMask, Option as PresentOption},
        render::{ConnectionExt as _, PictOp},
        xproto::{ConnectionExt as _, Rectangle},
    },
    x11_utils::X11Error,
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
            let request = conn.present_pixmap(
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
            self.submission = Some(u16::try_from(
                request.sequence_number() & u64::from(u16::MAX),
            )?);
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

    pub(crate) fn recover_present(&mut self, error: &X11Error) -> Result<bool> {
        if !self.present
            || Some(error.sequence) != self.submission
            || error.extension_name.as_deref() != Some("Present")
            || error.minor_opcode != u16::from(x11rb::protocol::present::PIXMAP_REQUEST)
            || error.error_kind != ErrorKind::Match
        {
            return Ok(false);
        }
        let back = self
            .conn
            .get_geometry(self.back.pixmap.context("back buffer lacks pixmap")?)?
            .reply()?;
        let output = self.conn.get_geometry(self.overlay)?.reply()?;
        ensure!(
            back.root == output.root
                && back.depth == output.depth
                && back.width == self.size.width
                && back.height == self.size.height,
            "Present recovery requires compatible, intact render buffers"
        );
        self.present = false;
        self.idle = true;
        self.complete = true;
        self.submission = None;
        tracing::warn!(
            ?error,
            "Present rejected submission; continuing with XRender"
        );
        Ok(true)
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
