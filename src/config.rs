use std::{fs, path::Path, time::Duration};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) opacity: u8,
    pub(crate) fade_ms: u16,
    pub(crate) blur_radius: u8,
    pub(crate) max_fps: u16,
    pub(crate) vsync: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opacity: 100,
            fade_ms: 180,
            blur_radius: 4,
            max_fps: 120,
            vsync: true,
        }
    }
}

impl Config {
    pub(crate) fn load(path: Option<&Path>) -> Result<Self> {
        let config = match path {
            Some(path) => Self::parse(
                &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
            )?,
            None => Self::default(),
        };
        Ok(config)
    }

    fn parse(text: &str) -> Result<Self> {
        let config: Self = toml::from_str(text).context("parsing compositor configuration")?;
        ensure!(config.opacity <= 100, "opacity must be between 0 and 100");
        ensure!(
            config.blur_radius <= 16,
            "blur_radius must be between 0 and 16"
        );
        ensure!(
            (1..=1000).contains(&config.max_fps),
            "max_fps must be between 1 and 1000"
        );
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
            "max_fps = 0",
            "max_fps = 1001",
            "fade_ms = -1",
            "opactiy = 80",
        ] {
            assert!(Config::parse(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn partial_configuration_preserves_defaults() {
        // Given a single setting, when parsed, then other settings retain defaults.
        let config = Config::parse("opacity = 73").unwrap();
        assert_eq!(config.opacity, 73);
        assert_eq!(config.fade_ms, 180);
        assert!(config.vsync);
    }
}
