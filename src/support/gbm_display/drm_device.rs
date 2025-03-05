use std::{
    ffi::CStr,
    fs::{File, OpenOptions},
    os::unix::io::{AsFd, BorrowedFd},
};

use anyhow::{Context as _, Result};
use drm::control::{
    self, connector, crtc, property::ValueType, Device as ControlDevice, ModeTypeFlags,
    PageFlipFlags,
};
use log::warn;

pub type FbHandle = drm::control::framebuffer::Handle;

#[derive(Debug)]
/// A simple wrapper for a device node.
pub struct Card(File);

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
        Ok(Card(
            options
                .open(path)
                .context(format!("While opening {path}"))?,
        ))
    }
}

pub struct DrmDevice {
    pub card: Card,
    pub connector: connector::Info,
    pub mode: control::Mode,
    pub crtc: crtc::Info,
    pub dpms_prop: Option<control::property::Info>,
}

impl AsFd for DrmDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.card.as_fd()
    }
}

impl drm::Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

impl DrmDevice {
    pub fn new() -> Result<Self> {
        let drm_device = Card::open().context("While opening DRM device")?;
        let res = drm_device
            .resource_handles()
            .context("While listing DRM resources handles")?;

        let connector = Self::find_connected_connector(&drm_device, &res)?;
        let mode = Self::find_preferred_mode(&connector)?;
        let crtc = Self::find_crtc(&drm_device, &connector)?;
        let dpms_prop = Self::get_dpms_property(&drm_device, &connector)?;

        Ok(Self {
            card: drm_device,
            connector,
            mode,
            crtc,
            dpms_prop,
        })
    }

    fn find_connected_connector(
        drm_device: &Card,
        res: &control::ResourceHandles,
    ) -> Result<connector::Info> {
        res.connectors()
            .iter()
            .filter_map(|h| drm_device.get_connector(*h, true).ok())
            .find(|c| c.state() == connector::State::Connected)
            .context("Cannot find connected connector")
    }

    fn find_preferred_mode(connector: &connector::Info) -> Result<control::Mode> {
        connector
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .cloned()
            .context("Cannot find preferred connector mode")
    }

    fn find_crtc(drm_device: &Card, connector: &connector::Info) -> Result<crtc::Info> {
        connector
            .encoders()
            .iter()
            .filter_map(|h| drm_device.get_encoder(*h).ok())
            .filter_map(|e| e.crtc())
            .filter_map(|c| drm_device.get_crtc(c).ok())
            .next()
            .context("Cannot get CRTC for connector")
    }

    fn get_dpms_property(
        drm_device: &Card,
        connector: &connector::Info,
    ) -> Result<Option<control::property::Info>> {
        let connector_props = drm_device
            .get_properties(connector.handle())
            .context("Cannot get connector properties")?;

        let connector_props = connector_props
            .as_hashmap(drm_device)
            .context("Cannot convert connector properties")?;
        let dpms_prop = connector_props.get("DPMS").cloned().filter(|p| p.mutable());
        if dpms_prop.is_none() {
            warn!("Connector does not support DPMS, screen will not turn off");
        }
        Ok(dpms_prop)
    }

    pub fn init_crtc(&self, framebuffer: FbHandle) -> Result<()> {
        self.set_crtc(
            self.crtc.handle(),
            Some(framebuffer),
            (0, 0),
            &[self.connector.handle()],
            Some(self.mode),
        )?;
        Ok(())
    }

    pub fn flip_and_wait(&self, fb: FbHandle) -> Result<()> {
        self.card
            .page_flip(self.crtc.handle(), fb, PageFlipFlags::EVENT, None)?;

        'outer: loop {
            let mut events = self.card.receive_events()?;
            for event in &mut events {
                if let control::Event::PageFlip(event) = event {
                    if event.crtc == self.crtc.handle() {
                        break 'outer;
                    }
                }
            }
        }
        Ok(())
    }

    // TODO write a wrapper for possible values & a getter
    pub fn set_dpms_property(&self, value: &CStr) -> Result<()> {
        if let Some(dpms_prop) = &self.dpms_prop {
            if let ValueType::Enum(enum_value) = dpms_prop.value_type() {
                for possible_value in enum_value.values().1 {
                    if possible_value.name() == value {
                        self.set_property(
                            self.connector.handle(),
                            dpms_prop.handle(),
                            possible_value.value(),
                        )
                        .context(format!("Cannot set DPMS property to {value:?}"))?;
                        return Ok(());
                    }
                }
                anyhow::bail!("Invalid DPMS value: {value:?}");
            } else {
                anyhow::bail!("DPMS property is not an enum");
            }
        } else {
            anyhow::bail!("DPMS property not found");
        }
    }
}
