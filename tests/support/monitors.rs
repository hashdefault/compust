use super::Desktop;
use anyhow::{Context, Result, ensure};
use x11rb::{
    CURRENT_TIME, NONE,
    protocol::{
        randr::{ConnectionExt as _, Crtc, GetCrtcInfoReply, SetConfig},
        xproto::ConnectionExt as _,
    },
};

pub(crate) struct Monitor {
    crtc: Crtc,
    original: GetCrtcInfoReply,
}

impl Monitor {
    pub(crate) fn new(desktop: &Desktop) -> Result<Self> {
        let version = desktop.conn.randr_query_version(1, 3)?.reply()?;
        ensure!(
            (version.major_version, version.minor_version) >= (1, 3),
            "monitor tests require RandR 1.3"
        );
        let resources = desktop
            .conn
            .randr_get_screen_resources_current(desktop.root)?
            .reply()?;
        let crtc = *resources.crtcs.first().context("Xvfb has no CRTC")?;
        let original = desktop
            .conn
            .randr_get_crtc_info(crtc, resources.config_timestamp)?
            .reply()?;
        ensure!(
            original.status == SetConfig::SUCCESS,
            "querying CRTC failed"
        );
        ensure!(original.mode != NONE, "Xvfb CRTC has no active mode");
        ensure!(!original.outputs.is_empty(), "Xvfb CRTC has no output");
        Ok(Self { crtc, original })
    }

    pub(crate) fn disable(&self, desktop: &Desktop) -> Result<()> {
        let timestamp = desktop
            .conn
            .randr_get_screen_resources_current(desktop.root)?
            .reply()?
            .config_timestamp;
        let result = desktop
            .conn
            .randr_set_crtc_config(
                self.crtc,
                CURRENT_TIME,
                timestamp,
                0,
                0,
                NONE,
                self.original.rotation,
                &[],
            )?
            .reply()?;
        ensure!(result.status == SetConfig::SUCCESS, "disabling CRTC failed");
        Ok(())
    }

    pub(crate) fn restore(&self, desktop: &Desktop) -> Result<()> {
        let timestamp = desktop
            .conn
            .randr_get_screen_resources_current(desktop.root)?
            .reply()?
            .config_timestamp;
        let result = desktop
            .conn
            .randr_set_crtc_config(
                self.crtc,
                CURRENT_TIME,
                timestamp,
                self.original.x,
                self.original.y,
                self.original.mode,
                self.original.rotation,
                &self.original.outputs,
            )?
            .reply()?;
        ensure!(result.status == SetConfig::SUCCESS, "restoring CRTC failed");
        Ok(())
    }
}

impl Desktop {
    pub(crate) fn resize_root(&self, size: (u16, u16)) -> Result<()> {
        self.conn
            .randr_set_screen_size(
                self.root,
                size.0,
                size.1,
                u32::from(size.0),
                u32::from(size.1),
            )?
            .check()?;
        let geometry = self.conn.get_geometry(self.root)?.reply()?;
        assert_eq!((geometry.width, geometry.height), size);
        Ok(())
    }
}
