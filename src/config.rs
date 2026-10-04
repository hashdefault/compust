use std::{env, ffi::OsString, fs, io, path::PathBuf, time::Duration};

use crate::rules::Rule;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) opacity: u8,
    pub(crate) fade_ms: u16,
    pub(crate) blur_radius: u8,
    /// How far a shadow spreads beyond its window, in pixels; zero draws none.
    pub(crate) shadow_radius: u8,
    pub(crate) shadow_offset_x: i8,
    pub(crate) shadow_offset_y: i8,
    /// How dark a shadow is where it is darkest, as a percentage.
    pub(crate) shadow_opacity: u8,
    pub(crate) max_fps: u16,
    pub(crate) vsync: bool,
    pub(crate) backend: Backend,
    /// Stop compositing while one opaque window covers the whole screen.
    pub(crate) unredirect_fullscreen: bool,
    /// Per-window rules, from `[[rules]]` tables in order.
    pub(crate) rules: Vec<Rule>,
}

/// What draws the frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Backend {
    /// The X server, through `XRender`.
    #[default]
    Xrender,
    /// OpenGL ES on the X server's GPU, sharing buffers through DRI3; it falls back to
    /// `XRender` when the server or driver cannot.
    Gl,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opacity: 100,
            fade_ms: 180,
            blur_radius: 4,
            shadow_radius: 0,
            shadow_offset_x: 0,
            shadow_offset_y: 0,
            shadow_opacity: 50,
            max_fps: 120,
            vsync: true,
            backend: Backend::Xrender,
            unredirect_fullscreen: false,
            rules: Vec::new(),
        }
    }
}

/// Where the configuration comes from: the `--config` file, or else the first
/// `compust/compust.toml` in the XDG configuration directories. The search runs again on every
/// load, so a reload reads what a restart would.
pub(crate) struct Source(Option<PathBuf>);

impl Source {
    pub(crate) fn new(explicit: Option<PathBuf>) -> Self {
        Self(explicit)
    }

    /// The configuration and the file it was read from; no file means built-in defaults.
    pub(crate) fn load(&self) -> Result<(Config, Option<PathBuf>)> {
        let path = match &self.0 {
            Some(path) => Some(path.clone()),
            None => discover(&candidates(
                env::var_os("XDG_CONFIG_HOME"),
                env::var_os("HOME"),
                env::var_os("XDG_CONFIG_DIRS"),
            ))?,
        };
        let config = match &path {
            Some(path) => fs::read_to_string(path)
                .map_err(anyhow::Error::from)
                .and_then(|text| Config::parse(&text))
                .with_context(|| format!("loading {}", path.display()))?,
            None => Config::default(),
        };
        Ok((config, path))
    }
}

/// Configuration files in XDG search order: `$XDG_CONFIG_HOME` (default `~/.config`), then each
/// `$XDG_CONFIG_DIRS` entry (default `/etc/xdg`). Relative directories are ignored, as the
/// base directory specification requires.
fn candidates(
    config_home: Option<OsString>,
    home: Option<OsString>,
    config_dirs: Option<OsString>,
) -> Vec<PathBuf> {
    let set = |value: Option<OsString>| value.filter(|value| !value.is_empty());
    let user = set(config_home)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| set(home).map(|home| PathBuf::from(home).join(".config")));
    let system = set(config_dirs).unwrap_or_else(|| "/etc/xdg".into());
    user.into_iter()
        .chain(env::split_paths(&system))
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("compust").join("compust.toml"))
        .collect()
}

/// The first candidate present. A dangling link counts as present, so reading it fails instead
/// of falling back to a later file or to the defaults; an inaccessible directory is an error too.
fn discover(candidates: &[PathBuf]) -> Result<Option<PathBuf>> {
    for path in candidates {
        match fs::symlink_metadata(path) {
            Ok(_) => return Ok(Some(path.clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => {
                return Err(error).with_context(|| format!("looking for {}", path.display()));
            }
        }
    }
    Ok(None)
}

impl Config {
    fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text).context("parsing compositor configuration")?;
        ensure!(config.opacity <= 100, "opacity must be between 0 and 100");
        ensure!(
            config.blur_radius <= 16,
            "blur_radius must be between 0 and 16"
        );
        ensure!(
            config.shadow_radius <= 64,
            "shadow_radius must be between 0 and 64"
        );
        for (name, offset) in [
            ("shadow_offset_x", config.shadow_offset_x),
            ("shadow_offset_y", config.shadow_offset_y),
        ] {
            ensure!(
                (-64..=64).contains(&offset),
                "{name} must be between -64 and 64"
            );
        }
        ensure!(
            config.shadow_opacity <= 100,
            "shadow_opacity must be between 0 and 100"
        );
        ensure!(
            (1..=1000).contains(&config.max_fps),
            "max_fps must be between 1 and 1000"
        );
        for (index, rule) in config.rules.iter().enumerate() {
            rule.validate(index + 1)?;
        }
        Ok(config)
    }

    pub(crate) fn frame_interval(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / u64::from(self.max_fps))
    }

    pub(crate) fn fade_duration(&self) -> Duration {
        Duration::from_millis(u64::from(self.fade_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_configuration_at_boundary() {
        // Given invalid external values, when parsed, then none enters the runtime.
        for input in [
            "opacity = 101",
            "opacity = -1",
            "blur_radius = 17",
            "shadow_radius = 65",
            "shadow_radius = -1",
            "shadow_offset_x = 65",
            "shadow_offset_y = -65",
            "shadow_opacity = 101",
            "max_fps = 0",
            "max_fps = 1001",
            "fade_ms = -1",
            "opactiy = 80",
            "backend = \"vulkan\"",
            "backend = \"GL\"",
            "unredirect_fullscreen = 1",
        ] {
            assert!(Config::parse(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn searches_xdg_directories_in_order() {
        // Given the XDG variables, when candidates are listed, then the user directory comes
        // first and relative or empty entries fall back as the specification requires.
        let os = |value: &str| Some(OsString::from(value));
        let file = |dir: &str| PathBuf::from(dir).join("compust/compust.toml");
        assert_eq!(
            candidates(os("/xdg"), os("/home/me"), os("/a:relative:/b")),
            [file("/xdg"), file("/a"), file("/b")]
        );
        assert_eq!(
            candidates(os(""), os("/home/me"), None),
            [file("/home/me/.config"), file("/etc/xdg")]
        );
        assert_eq!(
            candidates(os("relative"), os("/home/me"), os("")),
            [file("/home/me/.config"), file("/etc/xdg")]
        );
        assert_eq!(candidates(None, None, os("relative")), [] as [PathBuf; 0]);
    }

    #[test]
    fn discovery_takes_first_present_file_and_reports_dangling_links() {
        // Given candidates in temporary directories, when discovered, then the first present
        // file wins and a dangling link is reported instead of skipped.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.toml");
        let present = dir.path().join("present.toml");
        let later = dir.path().join("later.toml");
        fs::write(&present, "").unwrap();
        fs::write(&later, "").unwrap();
        assert_eq!(
            discover(&[missing.clone(), present.clone(), later]).unwrap(),
            Some(present)
        );
        assert_eq!(discover(std::slice::from_ref(&missing)).unwrap(), None);
        let dangling = dir.path().join("dangling.toml");
        std::os::unix::fs::symlink(&missing, &dangling).unwrap();
        let found = discover(std::slice::from_ref(&dangling)).unwrap();
        assert_eq!(found, Some(dangling.clone()));
        assert!(
            Source::new(found)
                .load()
                .is_err_and(|error| format!("{error:#}").contains("dangling.toml"))
        );
    }

    #[test]
    fn partial_configuration_preserves_defaults() {
        // Given a single setting, when parsed, then other settings retain defaults.
        let config = Config::parse("opacity = 73").unwrap();
        assert_eq!(config.opacity, 73);
        assert_eq!(config.fade_ms, 180);
        assert_eq!(config.shadow_radius, 0);
        assert_eq!(config.shadow_opacity, 50);
        assert!(config.vsync);
        assert!(!config.unredirect_fullscreen);
        assert!(
            Config::parse("unredirect_fullscreen = true")
                .unwrap()
                .unredirect_fullscreen
        );
        assert_eq!(config.backend, Backend::Xrender);
        assert_eq!(
            Config::parse("backend = \"gl\"").unwrap().backend,
            Backend::Gl
        );
        assert!(config.rules.is_empty());
    }

    #[test]
    fn rules_parse_in_order_and_an_invalid_one_rejects_the_file() {
        let config = Config::parse(
            "[[rules]]\nwindow_type = \"menu\"\nblur = false\n\n[[rules]]\nwm_class = \"Alacritty\"\nopacity = 90\n",
        )
        .unwrap();
        assert_eq!(config.rules.len(), 2);
        let shadows = Config::parse(
            "shadow_radius = 64\nshadow_offset_x = -64\nshadow_offset_y = 64\nshadow_opacity = 0\n",
        )
        .unwrap();
        assert_eq!(
            (
                shadows.shadow_radius,
                shadows.shadow_offset_x,
                shadows.shadow_offset_y
            ),
            (64, -64, 64)
        );
        let error = Config::parse("[[rules]]\nblur = false\n").unwrap_err();
        assert!(
            format!("{error:#}").contains("rule 1 needs wm_class"),
            "{error:#}"
        );
    }
}
