mod drm_device;
mod gbm_data;

use std::{rc::Rc, sync::Arc, thread::sleep, time::Duration};

use anyhow::{Context as _, Result};
use drm::control::{self, Device as ControlDevice, PageFlipFlags};
use glutin::{
    context::{ContextAttributesBuilder, NotCurrentContext, Priority},
    display::GetGlDisplay,
    prelude::GlDisplay,
};

use self::{drm_device::DrmDevice, gbm_data::GbmData};
use super::ApplicationContext;
use crate::{configuration::AppConfiguration, gl::FutureGlThreadContext};

fn create_gl_context(
    gbm_data: &GbmData,
    share_with: Option<&NotCurrentContext>,
    priority: Priority,
) -> Result<NotCurrentContext> {
    let mut builder = ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::Gles(None))
        .with_priority(priority);

    if let Some(share_context) = share_with {
        builder = builder.with_sharing(share_context);
    }

    unsafe {
        gbm_data
            .display
            .create_context(&gbm_data.gl_config, &builder.build(None))
            .context("Cannot create openGL context")
    }
}

pub fn start_gbm<T>(app_config: Arc<AppConfiguration>) -> Result<()>
where
    T: ApplicationContext + 'static,
{
    let drm_device = DrmDevice::new().context("While creating DrmDevice")?;
    let gbm_data = GbmData::new(drm_device)?;
    let (window_surface, surface) = gbm_data.create_gbm_window()?;

    let not_current_gl_context = create_gl_context(&gbm_data, None, Priority::Medium)?;

    let gl = FutureGlThreadContext::new(
        Some(window_surface),
        not_current_gl_context,
        gbm_data.gl_config.display(),
    );

    let bg_context = create_gl_context(&gbm_data, Some(gl.get_context()), Priority::Low)?;

    let gl = gl
        .activate()
        .context("Cannot activate main GL context on surface")?;
    let bg_gl = FutureGlThreadContext::new(None, bg_context, gbm_data.gl_config.display());

    gl.swap_buffers().context("Cannot swap buffers")?;

    let mut bo = unsafe { surface.lock_front_buffer() }
        .context("Cannot lock front buffer")?;
    let bpp = bo.bpp();
    let mut fb = gbm_data
        .add_framebuffer(&bo, bpp, bpp)
        .context("Cannot get framebuffer")?;
    gbm_data
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

            let next_bo = unsafe { surface.lock_front_buffer() }
                .context("Cannot lock front buffer")?;
            let next_fb = gbm_data
                .add_framebuffer(&next_bo, bpp, bpp)
                .context("Cannot get framebuffer")?;
            gbm_data
                .page_flip(
                    gbm_data.device.crtc.handle(),
                    next_fb,
                    PageFlipFlags::EVENT,
                    None,
                )
                .context("Cannot request pageflip")?;

            'outer: loop {
                let mut events = gbm_data
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
                .destroy_framebuffer(fb)
                .context("Cannot free old framebuffer")?;
            fb = next_fb;
        } else {
            gbm_data.device.set_dpms_property(c"Standby")?;
            sleep(Duration::from_secs(60));
        }
    }
}
