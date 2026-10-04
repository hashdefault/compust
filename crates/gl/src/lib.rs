//! GPU access for the Compust compositor: an OpenGL ES context on an EGL device, textures
//! imported from dma-bufs without copying, and drawing between them. The compositor crate
//! forbids unsafe code; every unsafe call its GPU renderer needs lives here, each with the
//! reason it is sound.

mod device;
mod draw;
mod texture;

pub use device::{Device, Gpu};
pub use draw::{Frame, Mode, Placement, Rect};
pub use texture::{Dmabuf, MOD_INVALID, Plane, Target, Texture, fourcc};

#[cfg(test)]
mod tests;
