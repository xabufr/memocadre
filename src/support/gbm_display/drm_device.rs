use std::{
    fs::{File, OpenOptions},
    os::unix::io::{AsFd, BorrowedFd},
    rc::Rc,
};

use anyhow::{Context as _, Result};
use drm::control::{self, connector, crtc, Device as ControlDevice, ModeTypeFlags};
use log::warn;

#[derive(Debug, Clone)]
/// A simple wrapper for a device node.
pub struct Card(Rc<File>);

/// Implementing [`AsFd`] is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling [`File::as_fd()`] on the inner
/// [`File`].
impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// With [`AsFd`] implemented, we can now implement [`drm::Device`].
impl drm::Device for Card {}
impl ControlDevice for Card {}

impl Card {
    /// Simple helper method for opening a [`Card`].
    fn open() -> Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true);
        options.write(true);

        // The normal location of the primary device node on Linux
        let path = "/dev/dri/card0";
        Ok(Card(Rc::new(
            options
                .open(path)
                .context(format!("While opening {path}"))?,
        )))
    }
}

pub struct DrmDevice {
    pub card: Card,
    pub connector: connector::Info,
    pub mode: control::Mode,
    pub crtc: crtc::Info,
    pub dpms_prop: Option<control::property::Info>,
}

impl DrmDevice {
    pub fn new() -> Result<Self> {
        let drm_device = Card::open().context("While opening DRM device")?;
        let res = drm_device
            .resource_handles()
            .context("While listing DRM resources handles")?;
        let connector = res
            .connectors()
            .iter()
            .flat_map(|h| drm_device.get_connector(*h, true))
            .find(|c| c.state() == connector::State::Connected)
            .context("Cannot find connected connector")?;
        let mode = connector
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .context("Cannot find prefered connector mode")?
            .clone();
        let crtc = connector
            .encoders()
            .iter()
            .flat_map(|h| drm_device.get_encoder(*h))
            .flat_map(|e| e.crtc())
            .flat_map(|c| drm_device.get_crtc(c))
            .next()
            .context("Cannot get CRTC")?;

        let connector_props = drm_device
            .get_properties(connector.handle())
            .context("Cannot get connector properties")?;

        let connector_props = connector_props
            .as_hashmap(&drm_device)
            .context("Cannot convert connector properties")?;
        let dpms_prop = connector_props.get("DPMS").cloned();
        if dpms_prop.is_none() {
            warn!("Connector does not support DPMS, screen will not turn off");
        }
        Ok(Self {
            card: drm_device,
            connector,
            mode,
            crtc,
            dpms_prop,
        })
    }
}
