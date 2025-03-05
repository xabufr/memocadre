use std::{ffi::c_void, num::NonZeroU32, os::fd::AsFd, ptr::NonNull};

use anyhow::{Context as _, Result};
use gbm::{AsRaw, BufferObjectFlags};
use glutin::{
    config::{Api, ConfigTemplateBuilder},
    display::GlDisplay,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use log::debug;
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::drm_device::DrmDevice;

pub struct GbmData {
    pub device: gbm::Device<DrmDevice>,
    pub display: glutin::display::Display,
    pub gl_config: glutin::config::Config,
}

pub type GbmWindow = (
    glutin::surface::Surface<glutin::surface::WindowSurface>,
    gbm::Surface<()>,
);

impl AsFd for GbmData {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.device.as_fd()
    }
}
impl drm::Device for GbmData {}
impl drm::control::Device for GbmData {}

impl GbmData {
    pub fn new(drm_device: DrmDevice) -> Result<Self> {
        let (width, height) = drm_device.mode.size();
        debug!(
            "Will start DRM rendering with {width}x{height}@{} resolution",
            drm_device.mode.vrefresh()
        );

        let device = gbm::Device::new(drm_device).context("Cannot open GBM device")?;
        let display = unsafe {
            let ptr: NonNull<c_void> =
                NonNull::new(device.as_raw() as *mut c_void).context("device pointer is null")?;
            let display = RawDisplayHandle::Gbm(GbmDisplayHandle::new(ptr));
            glutin::display::Display::new(display, glutin::display::DisplayApiPreference::Egl)
                .context("Cannot initialize glutin display")?
        };
        let gl_config = unsafe {
            display
                .find_configs(
                    ConfigTemplateBuilder::new()
                        .prefer_hardware_accelerated(Some(true))
                        .with_api(Api::GLES2)
                        .build(),
                )
                .context("Cannot find config")?
                .next()
                .context("No available config found")?
        };

        Ok(Self {
            device,
            display,
            gl_config,
        })
    }

    pub fn create_gbm_window(&self) -> Result<GbmWindow> {
        let (width, height) = self.device.mode.size();
        debug!("Using gl config: {:?}", self.gl_config);
        let (window_surface, gbm_surface) = unsafe {
            let gbm_surface = self
                .device
                .create_surface::<()>(
                    width as _,
                    height as _,
                    gbm::Format::Xrgb8888,
                    BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
                )
                .context("Cannot create GBM surface")?;
            let window_handle = RawWindowHandle::Gbm(GbmWindowHandle::new(
                NonNull::new(gbm_surface.as_raw() as *mut c_void).context("GBM surface is null")?,
            ));
            let surface = self
                .display
                .create_window_surface(
                    &self.gl_config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        window_handle,
                        NonZeroU32::new(width as _).unwrap(),
                        NonZeroU32::new(height as _).unwrap(),
                    ),
                )
                .context("Cannot create window surface")?;
            (surface, gbm_surface)
        };
        Ok((window_surface, gbm_surface))
    }
}
