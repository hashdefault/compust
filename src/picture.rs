use anyhow::Result;
use std::rc::Rc;
use x11rb::{
    connection::Connection,
    protocol::{
        render::{ConnectionExt as _, CreatePictureAux, Pictformat, Picture as PictureId, Repeat},
        xproto::{ConnectionExt as _, Drawable, Pixmap},
    },
    rust_connection::RustConnection,
};

pub(crate) struct Picture {
    conn: Rc<RustConnection>,
    pub(crate) id: PictureId,
    pub(crate) pixmap: Option<Pixmap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Size {
    pub(crate) width: u16,
    pub(crate) height: u16,
}

#[derive(Clone, Copy)]
pub(crate) struct Format {
    pub(crate) depth: u8,
    pub(crate) id: Pictformat,
}

impl Picture {
    pub(crate) fn borrowed(
        conn: &Rc<RustConnection>,
        drawable: Drawable,
        format: Pictformat,
    ) -> Result<Self> {
        let id = conn.generate_id()?;
        conn.render_create_picture(
            id,
            drawable,
            format,
            &CreatePictureAux::new()
                .subwindowmode(x11rb::protocol::xproto::SubwindowMode::INCLUDE_INFERIORS),
        )?
        .check()?;
        Ok(Self {
            conn: Rc::clone(conn),
            id,
            pixmap: None,
        })
    }

    pub(crate) fn buffer(
        conn: &Rc<RustConnection>,
        drawable: Drawable,
        layout: (Size, Format),
    ) -> Result<Self> {
        let (size, format) = layout;
        let pixmap = conn.generate_id()?;
        conn.create_pixmap(format.depth, pixmap, drawable, size.width, size.height)?
            .check()?;
        let picture = Self::borrowed(conn, pixmap, format.id);
        match picture {
            Ok(mut picture) => {
                picture.pixmap = Some(pixmap);
                Ok(picture)
            }
            Err(error) => {
                conn.free_pixmap(pixmap)?;
                Err(error)
            }
        }
    }

    pub(crate) fn repeat(&self, repeat: Repeat) -> Result<()> {
        self.conn
            .render_change_picture(
                self.id,
                &x11rb::protocol::render::ChangePictureAux::new().repeat(repeat),
            )?
            .check()?;
        Ok(())
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        if let Err(error) = self
            .conn
            .render_free_picture(self.id)
            .map(x11rb::cookie::VoidCookie::ignore_error)
        {
            tracing::debug!(%error, "picture cleanup failed");
        }
        if let Some(pixmap) = self.pixmap
            && let Err(error) = self
                .conn
                .free_pixmap(pixmap)
                .map(x11rb::cookie::VoidCookie::ignore_error)
        {
            tracing::debug!(%error, "pixmap cleanup failed");
        }
    }
}
