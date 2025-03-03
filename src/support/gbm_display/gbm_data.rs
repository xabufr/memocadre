use std::{ffi::c_void, num::NonZeroU32, ptr::NonNull};

use anyhow::{Context as _, Result};
use gbm::{AsRaw, BufferObjectFlags};
use glutin::{
    config::{Api, ConfigTemplateBuilder},
    display::GlDisplay,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use log::debug;
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::drm_device::{Card, DrmDevice};

pub struct GbmData {
    pub device: gbm::Device<Card>,
    pub display: glutin::display::Display,
    pub gl_config: glutin::config::Config,
    pub surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    pub window: RawWindowHandle,
    pub gbm_surface: gbm::Surface<()>,
}

impl GbmData {
    pub fn new(drm_device: &DrmDevice) -> Result<Self> {
        let (width, height) = drm_device.mode.size();
        debug!(
            "Will start DRM rendering with {width}x{height}@{} resolution",
            drm_device.mode.vrefresh()
        );

        let device = gbm::Device::new(drm_device.card.clone()).context("Cannot open GBM device")?;
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

        debug!("Using gl config: {gl_config:?}");
        let (surface, window, gbm_surface) = unsafe {
            let gbm_surface = device
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
            let surface = display
                .create_window_surface(
                    &gl_config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        window_handle,
                        NonZeroU32::new(width as _).unwrap(),
                        NonZeroU32::new(height as _).unwrap(),
                    ),
                )
                .context("Cannot create window surface")?;
            (surface, window_handle, gbm_surface)
        };

        Ok(Self {
            device,
            display,
            gl_config,
            surface,
            window,
            gbm_surface,
        })
    }
}
