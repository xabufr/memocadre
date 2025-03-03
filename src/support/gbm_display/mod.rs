mod drm_device;
mod gbm_data;

use std::{rc::Rc, sync::Arc, thread::sleep, time::Duration};

use anyhow::{Context as _, Result};
use drm::control::{self, property::ValueType, Device as ControlDevice, PageFlipFlags};
use glutin::{context::ContextAttributesBuilder, display::GetGlDisplay, prelude::GlDisplay};

use self::{drm_device::DrmDevice, gbm_data::GbmData};
use super::ApplicationContext;
use crate::{configuration::AppConfiguration, gl::FutureGlThreadContext};

pub fn start_gbm<T>(app_config: Arc<AppConfiguration>) -> Result<()>
where
    T: ApplicationContext + 'static,
{
    let drm_device = DrmDevice::new().context("While creating DrmDevice")?;
    let gbm_data = GbmData::new(drm_device)?;

    let not_current_gl_context = unsafe {
        gbm_data
            .display
            .create_context(
                &gbm_data.gl_config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .build(Some(gbm_data.window)),
            )
            .context("Cannot create openGL context")?
    };

    let gl = FutureGlThreadContext::new(
        Some(gbm_data.surface),
        not_current_gl_context,
        gbm_data.gl_config.display(),
    );

    let bg_context = unsafe {
        gbm_data
            .display
            .create_context(
                &gbm_data.gl_config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .with_sharing(gl.get_context())
                    .with_priority(glutin::context::Priority::Low)
                    .build(Some(gbm_data.window)),
            )
            .context("Cannot create BG openGL context")?
    };

    let gl = gl
        .activate()
        .context("Cannot activate main GL context on surface")?;
    let bg_gl = FutureGlThreadContext::new(None, bg_context, gbm_data.gl_config.display());

    gl.swap_buffers().context("Cannot swap buffers")?;

    let mut bo =
        unsafe { gbm_data.gbm_surface.lock_front_buffer() }.context("Cannot lock front buffer")?;
    let bpp = bo.bpp();
    let mut fb = gbm_data
        .device
        .add_framebuffer(&bo, bpp, bpp)
        .context("Cannot get framebuffer")?;
    gbm_data
        .device
        .set_crtc(
            gbm_data.device.crtc.handle(),
            Some(fb),
            (0, 0),
            &[gbm_data.device.connector.handle()],
            Some(gbm_data.device.mode),
        )
        .context("Cannot setup DRM device CRTC")?;

    let mut app = T::new(app_config, Rc::clone(&gl), bg_gl).context("Cannot create application")?;
    let enabled = false;
    loop {
        if enabled {
            app.draw_frame().context("Error while drawing a frame")?;

            let next_bo = unsafe { gbm_data.gbm_surface.lock_front_buffer() }
                .context("Cannot lock front buffer")?;
            let next_fb = gbm_data
                .device
                .add_framebuffer(&next_bo, bpp, bpp)
                .context("Cannot get framebuffer")?;
            gbm_data
                .device
                .page_flip(
                    gbm_data.device.crtc.handle(),
                    next_fb,
                    PageFlipFlags::EVENT,
                    None,
                )
                .context("Cannot request pageflip")?;

            'outer: loop {
                let mut events = gbm_data
                    .device
                    .receive_events()
                    .context("Cannot read DRM device events")?;
                for event in &mut events {
                    if let control::Event::PageFlip(event) = event {
                        if event.crtc == gbm_data.device.crtc.handle() {
                            break 'outer;
                        }
                    }
                }
            }
            drop(bo);
            bo = next_bo;
            gbm_data
                .device
                .destroy_framebuffer(fb)
                .context("Cannot free old framebuffer")?;
            fb = next_fb;
        } else if let Some(dpms_prop) = &gbm_data.device.dpms_prop {
            if let ValueType::Enum(value) = dpms_prop.value_type() {
                for value in value.values().1 {
                    if value.name() == c"Standby" {
                        gbm_data
                            .device
                            .set_property(
                                gbm_data.device.connector.handle(),
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
