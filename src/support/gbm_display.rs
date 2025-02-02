use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    num::NonZeroU32,
    os::unix::io::{AsFd, BorrowedFd},
    ptr::NonNull,
    sync::Arc,
};

use anyhow::{Context as _, Result};
use drm::{
    control::{self, connector, Device as ControlDevice, ModeTypeFlags, PageFlipFlags},
    Device as DrmDevice,
};
use gbm::{AsRaw, BufferObjectFlags};
use glow::Context;
use glutin::{
    config::ConfigTemplateBuilder,
    context::ContextAttributesBuilder,
    display::GlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use log::debug;
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};
use vek::Rect;

use super::ApplicationContext;
use crate::{
    configuration::Conf,
    gl::{GlContext, GlContextInner},
};

#[derive(Debug)]
/// A simple wrapper for a device node.
struct Card(File);

/// Implementing [`AsFd`] is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling [`File::as_fd()`] on the inner
/// [`File`].
impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// With [`AsFd`] implemented, we can now implement [`drm::Device`].
impl DrmDevice for Card {}
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
pub fn start_gbm<T>(app_config: Arc<Conf>) -> Result<()>
where
    T: ApplicationContext + 'static,
{
    let devices = glutin::api::egl::device::Device::query_devices()
        .context("Failed to query devices")?
        .collect::<Vec<_>>();
    debug!("found devices: {devices:#?}");

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
        .context("Cannot find prefered connector mode")?;
    let crtc = connector
        .encoders()
        .iter()
        .flat_map(|h| drm_device.get_encoder(*h))
        .flat_map(|e| e.crtc())
        .flat_map(|c| drm_device.get_crtc(c))
        .next()
        .context("Cannot get CRTC")?;

    let (width, height) = mode.size();
    let device = gbm::Device::new(drm_device).context("Cannot open GBM device")?;
    let display = unsafe {
        let ptr: NonNull<c_void> =
            NonNull::new(device.as_raw() as *mut c_void).context("device pointer is null")?;
        let display = RawDisplayHandle::Gbm(GbmDisplayHandle::new(ptr));
        glutin::display::Display::new(display, glutin::display::DisplayApiPreference::Egl)
            .context("Cannot initialize glutin display")?
    };
    let config = unsafe {
        let configs = display
            .find_configs(
                ConfigTemplateBuilder::new()
                    .prefer_hardware_accelerated(Some(true))
                    .build(),
            )
            .context("Cannot find config")?;
        let configs = configs.collect::<Vec<_>>();
        for config in &configs {
            println!("config: {:?}", config);
            println!(
                "config APIs: {:?}",
                config.api().iter_names().collect::<Vec<_>>()
            );
            println!("config hardware: {:?}", config.hardware_accelerated());
            println!("color: {:?}", config.color_buffer_type());
            println!("float pixels: {:?}", config.float_pixels());
            println!("samples: {:?}", config.num_samples());
        }
        configs
            .into_iter()
            .next()
            .context("No available config found")?
    };
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
                &config,
                &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                    window_handle,
                    NonZeroU32::new(width as _).unwrap(),
                    NonZeroU32::new(height as _).unwrap(),
                ),
            )
            .context("Cannot create window surface")?;
        (surface, window_handle, gbm_surface)
    };
    let not_current_gl_context = unsafe {
        display
            .create_context(
                &config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .build(Some(window)),
            )
            .context("Cannot create openGL context")?
    };

    let bg_context = unsafe {
        display
            .create_context(
                &config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .with_sharing(&not_current_gl_context)
                    .with_priority(glutin::context::Priority::Low)
                    .build(Some(window)),
            )
            .context("Cannot create BG openGL context")?
    };

    let current_context = not_current_gl_context
        .make_current(&surface)
        .context("Cannot activate GL context on surface")?;
    surface
        .swap_buffers(&current_context)
        .context("Cannot swap buffers")?;

    let mut bo = unsafe { gbm_surface.lock_front_buffer() }.context("Cannot lock front buffer")?;
    let bpp = bo.bpp();
    let mut fb = device
        .add_framebuffer(&bo, bpp, bpp)
        .context("Cannot get framebuffer")?;
    device
        .set_crtc(
            crtc.handle(),
            Some(fb),
            (0, 0),
            &[connector.handle()],
            Some(*mode),
        )
        .context("Cannot setup DRM device CRTC")?;

    let gl = unsafe { Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
    let gl = GlContextInner::new(gl, Rect::new(0, 0, width as _, height as _));

    let gl_bg =
        unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };

    let mut app = T::new(app_config, GlContext::clone(&gl), bg_context, gl_bg)
        .context("Cannot create application")?;
    loop {
        app.draw_frame().context("Error while drawing a frame")?;

        surface
            .swap_buffers(&current_context)
            .context("Cannot swap buffers on surface")?;

        let next_bo =
            unsafe { gbm_surface.lock_front_buffer() }.context("Cannot lock front buffer")?;
        let next_fb = device
            .add_framebuffer(&next_bo, bpp, bpp)
            .context("Cannot get framebuffer")?;
        device
            .page_flip(crtc.handle(), next_fb, PageFlipFlags::EVENT, None)
            .context("Cannot request pageflip")?;

        'outer: loop {
            let mut events = device
                .receive_events()
                .context("Cannot read DRM device events")?;
            for event in &mut events {
                if let control::Event::PageFlip(event) = event {
                    if event.crtc == crtc.handle() {
                        break 'outer;
                    }
                }
            }
        }
        drop(bo);
        bo = next_bo;
        device
            .destroy_framebuffer(fb)
            .context("Cannot free old framebuffer")?;
        fb = next_fb;
    }
}
