mod drm_device;

use std::{
    ffi::c_void,
    num::NonZeroU32,
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use anyhow::{Context as _, Result};
use drm::
    control::{
        self, property::ValueType, Device as ControlDevice, PageFlipFlags,
    }
;
use gbm::{AsRaw, BufferObjectFlags};
use glutin::{
    config::{Api, ConfigTemplateBuilder},
    context::ContextAttributesBuilder,
    display::{GetGlDisplay as _, GlDisplay},
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use log::debug;
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::ApplicationContext;
use crate::{configuration::AppConfiguration, gl::FutureGlThreadContext};
use self::drm_device::DrmDevice;

pub fn start_gbm<T>(app_config: Arc<AppConfiguration>) -> Result<()>
where
    T: ApplicationContext + 'static,
{
    let drm_device = DrmDevice::new().context("While creating DrmDevice")?;

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
    let not_current_gl_context = unsafe {
        display
            .create_context(
                &gl_config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .build(Some(window)),
            )
            .context("Cannot create openGL context")?
    };

    let gl = FutureGlThreadContext::new(Some(surface), not_current_gl_context, gl_config.display());

    let bg_context = unsafe {
        display
            .create_context(
                &gl_config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .with_sharing(gl.get_context())
                    .with_priority(glutin::context::Priority::Low)
                    .build(Some(window)),
            )
            .context("Cannot create BG openGL context")?
    };

    let gl = gl
        .activate()
        .context("Cannot activate main GL context on surface")?;
    let bg_gl = FutureGlThreadContext::new(None, bg_context, gl_config.display());

    gl.swap_buffers().context("Cannot swap buffers")?;

    let mut bo = unsafe { gbm_surface.lock_front_buffer() }.context("Cannot lock front buffer")?;
    let bpp = bo.bpp();
    let mut fb = drm_device
        .card
        .add_framebuffer(&bo, bpp, bpp)
        .context("Cannot get framebuffer")?;
    drm_device
        .card
        .set_crtc(
            drm_device.crtc.handle(),
            Some(fb),
            (0, 0),
            &[drm_device.connector.handle()],
            Some(drm_device.mode),
        )
        .context("Cannot setup DRM device CRTC")?;

    let mut app = T::new(app_config, Rc::clone(&gl), bg_gl).context("Cannot create application")?;
    let enabled = false;
    loop {
        if enabled {
            app.draw_frame().context("Error while drawing a frame")?;

            let next_bo =
                unsafe { gbm_surface.lock_front_buffer() }.context("Cannot lock front buffer")?;
            let next_fb = drm_device
                .card
                .add_framebuffer(&next_bo, bpp, bpp)
                .context("Cannot get framebuffer")?;
            drm_device
                .card
                .page_flip(
                    drm_device.crtc.handle(),
                    next_fb,
                    PageFlipFlags::EVENT,
                    None,
                )
                .context("Cannot request pageflip")?;

            'outer: loop {
                let mut events = drm_device
                    .card
                    .receive_events()
                    .context("Cannot read DRM device events")?;
                for event in &mut events {
                    if let control::Event::PageFlip(event) = event {
                        if event.crtc == drm_device.crtc.handle() {
                            break 'outer;
                        }
                    }
                }
            }
            drop(bo);
            bo = next_bo;
            drm_device
                .card
                .destroy_framebuffer(fb)
                .context("Cannot free old framebuffer")?;
            fb = next_fb;
        } else if let Some(dpms_prop) = &drm_device.dpms_prop {
            if let ValueType::Enum(value) = dpms_prop.value_type() {
                for value in value.values().1 {
                    if value.name() == c"Standby" {
                        drm_device
                            .card
                            .set_property(
                                drm_device.connector.handle(),
                                dpms_prop.handle(),
                                value.value(),
                            )
                            .context("Cannot set DPMS property")?;
                        break;
                    }
                }
            }
            sleep(Duration::from_secs(60));
        }
    }
}
