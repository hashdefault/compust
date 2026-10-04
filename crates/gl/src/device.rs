use crate::draw::Programs;
use anyhow::{Context as _, Result, bail, ensure};
use khronos_egl as egl;
use std::{
    ffi::{CStr, c_char, c_void},
    os::unix::fs::MetadataExt,
    ptr::null_mut,
    rc::Rc,
};

pub(crate) type Egl = egl::DynamicInstance<egl::EGL1_5>;

const PLATFORM_DEVICE: egl::Enum = 0x313F;
const DRM_DEVICE_FILE: egl::Int = 0x3233;
const DRM_RENDER_NODE_FILE: egl::Int = 0x3377;

type QueryDevices =
    unsafe extern "system" fn(egl::Int, *mut *mut c_void, *mut egl::Int) -> egl::Boolean;
type QueryDeviceString = unsafe extern "system" fn(*mut c_void, egl::Int) -> *const c_char;
pub(crate) type ImageTargetTexture = unsafe extern "system" fn(u32, *mut c_void);

/// Which GPU to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    /// The GPU behind a DRM device node, by the node's device number (`st_rdev`), such as
    /// the node the X server hands out through DRI3.
    Drm(u64),
    /// Mesa's software rasterizer, for tests without a GPU.
    Software,
}

/// The EGL display and OpenGL ES context every GPU object belongs to. Every GL call goes
/// through `gl`, which makes this context current on the thread first, so calls reach it
/// even while another context exists, such as one replacing it.
pub(crate) struct Inner {
    pub(crate) egl: Egl,
    pub(crate) display: egl::Display,
    pub(crate) context: egl::Context,
    functions: glow::Context,
    pub(crate) image_target_texture: ImageTargetTexture,
    /// Whether dma-buf imports may name a format modifier.
    pub(crate) modifiers: bool,
}

impl Inner {
    /// The GL functions, with this context current on the thread.
    pub(crate) fn gl(&self) -> Result<&glow::Context> {
        if !self.is_current() {
            self.egl
                .make_current(self.display, None, None, Some(self.context))
                .context("making the GL context current")?;
        }
        Ok(&self.functions)
    }

    fn is_current(&self) -> bool {
        self.egl
            .get_current_context()
            .is_some_and(|current| current.as_ptr() == self.context.as_ptr())
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // EGL gives every user of a device in the process the same display, such as a
        // renderer and the one replacing it, so the display is never terminated; it goes with
        // the process. Errors cannot be reported from here.
        if self.is_current() {
            let _unbound = self.egl.make_current(self.display, None, None, None);
        }
        let _destroyed = self.egl.destroy_context(self.display, self.context);
    }
}

/// An OpenGL ES context on one GPU, current on this thread for as long as it lives. GPU
/// objects keep it alive, so they never outlive the context they belong to.
pub struct Gpu {
    pub(crate) programs: Programs,
    pub(crate) inner: Rc<Inner>,
}

impl Gpu {
    pub fn open(device: Device) -> Result<Self> {
        // SAFETY: libEGL is the system's EGL implementation, which provides the EGL 1.5 entry
        // points the instance loads under their standard names and signatures.
        let egl = unsafe { Egl::load_required() }.context("loading libEGL")?;
        let chosen = find(&egl, device)?;
        // SAFETY: `chosen` is a device that eglQueryDevicesEXT returned, which is the native
        // display EGL_EXT_platform_device expects.
        let display =
            unsafe { egl.get_platform_display(PLATFORM_DEVICE, chosen, &[egl::ATTRIB_NONE]) }
                .context("opening an EGL display on the device")?;
        egl.initialize(display).context("initializing EGL")?;
        let (context, modifiers, gl, image_target_texture) = context(&egl, display)?;
        let inner = Rc::new(Inner {
            egl,
            display,
            context,
            functions: gl,
            image_target_texture,
            modifiers,
        });
        let programs = Programs::new(&inner)?;
        Ok(Self { programs, inner })
    }

    /// The GL renderer's name, for diagnostics.
    pub fn renderer(&self) -> Result<String> {
        use glow::HasContext as _;
        let gl = self.inner.gl()?;
        // SAFETY: `gl` made the context current, and RENDERER is a string query.
        Ok(unsafe { gl.get_parameter_string(glow::RENDERER) })
    }
}

/// Create and make current an OpenGL ES context on `display`, with what imports need: the
/// context, whether imports may name modifiers, its GL functions, and the function that binds
/// an EGL image to a texture.
fn context(
    egl: &Egl,
    display: egl::Display,
) -> Result<(egl::Context, bool, glow::Context, ImageTargetTexture)> {
    let extensions = egl.query_string(Some(display), egl::EXTENSIONS)?;
    let extensions = extensions.to_string_lossy();
    for required in [
        "EGL_KHR_surfaceless_context",
        "EGL_KHR_no_config_context",
        "EGL_EXT_image_dma_buf_import",
    ] {
        ensure!(
            extensions.split(' ').any(|name| name == required),
            "EGL lacks {required}"
        );
    }
    let modifiers = extensions
        .split(' ')
        .any(|name| name == "EGL_EXT_image_dma_buf_import_modifiers");
    egl.bind_api(egl::OPENGL_ES_API)?;
    // SAFETY: a null config is EGL_NO_CONFIG_KHR, which EGL_KHR_no_config_context accepts in
    // place of a config.
    let config = unsafe { egl::Config::from_ptr(null_mut()) };
    let context = [3, 2]
        .into_iter()
        .find_map(|version| {
            let attributes = [egl::CONTEXT_MAJOR_VERSION, version, egl::NONE];
            egl.create_context(display, config, None, &attributes).ok()
        })
        .context("creating an OpenGL ES context")?;
    let current = egl
        .make_current(display, None, None, Some(context))
        .context("making the context current without a surface");
    // SAFETY: GL_OES_EGL_image defines glEGLImageTargetTexture2DOES with this signature.
    let image_target_texture =
        unsafe { extension::<ImageTargetTexture>(egl, "glEGLImageTargetTexture2DOES") };
    let image_target_texture = match current.and(image_target_texture) {
        Ok(function) => function,
        Err(error) => {
            let _destroyed = egl.destroy_context(display, context);
            return Err(error);
        }
    };
    let loader = |name: &CStr| {
        name.to_str()
            .ok()
            .and_then(|name| egl.get_proc_address(name))
            .map_or(std::ptr::null(), |address| address as *const c_void)
    };
    // SAFETY: the context is current, and the loader returns EGL's entry points for it, or
    // null for functions EGL lacks, as glow expects.
    let gl = unsafe { glow::Context::from_loader_function_cstr(loader) };
    Ok((context, modifiers, gl, image_target_texture))
}

/// The EGL device `device` names.
fn find(egl: &Egl, device: Device) -> Result<*mut c_void> {
    // SAFETY: EGL_EXT_device_enumeration defines eglQueryDevicesEXT with this signature.
    let query: QueryDevices = unsafe { extension(egl, "eglQueryDevicesEXT") }?;
    // SAFETY: EGL_EXT_device_query defines eglQueryDeviceStringEXT with this signature.
    let string: QueryDeviceString = unsafe { extension(egl, "eglQueryDeviceStringEXT") }?;
    let mut count = 0;
    // SAFETY: with no array, EGL only writes the number of devices to `count`.
    let listed = unsafe { query(0, null_mut(), &raw mut count) };
    ensure!(listed == egl::TRUE, "EGL could not count its devices");
    let mut devices = vec![null_mut(); usize::try_from(count)?];
    // SAFETY: `devices` has room for `count` entries, the most EGL writes.
    let listed = unsafe { query(count, devices.as_mut_ptr(), &raw mut count) };
    ensure!(listed == egl::TRUE, "EGL could not list its devices");
    devices.truncate(usize::try_from(count)?);
    let text = |candidate, name| {
        // SAFETY: `candidate` came from eglQueryDevicesEXT, and the query returns null or a
        // string the device owns.
        let value = unsafe { string(candidate, name) };
        if value.is_null() {
            return None;
        }
        // SAFETY: a non-null result is a NUL-terminated string that lives with the device.
        Some(
            unsafe { CStr::from_ptr(value) }
                .to_string_lossy()
                .into_owned(),
        )
    };
    let matches = |candidate| match device {
        Device::Software => text(candidate, egl::EXTENSIONS).is_some_and(|names| {
            names
                .split(' ')
                .any(|name| name == "EGL_MESA_device_software")
        }),
        Device::Drm(number) => [DRM_DEVICE_FILE, DRM_RENDER_NODE_FILE]
            .into_iter()
            .filter_map(|name| text(candidate, name))
            .filter_map(|path| std::fs::metadata(path).ok())
            .any(|node| node.rdev() == number),
    };
    match devices.into_iter().find(|candidate| matches(*candidate)) {
        Some(found) => Ok(found),
        None => bail!("no EGL device matches {device:?}"),
    }
}

/// The EGL or GL extension function `name`.
///
/// # Safety
///
/// `T` must be the function pointer type that the extension specification gives `name`.
pub(crate) unsafe fn extension<T: Copy>(egl: &Egl, name: &str) -> Result<T> {
    let address = egl
        .get_proc_address(name)
        .with_context(|| format!("EGL lacks {name}"))?;
    ensure!(
        size_of::<T>() == size_of_val(&address),
        "{name} is not a function pointer"
    );
    // SAFETY: the caller guarantees that `T` is the function's pointer type, and both are
    // function pointers of the same size.
    Ok(unsafe { std::mem::transmute_copy(&address) })
}
