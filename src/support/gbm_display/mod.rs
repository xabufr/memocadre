mod drm_device;
mod gbm_data;
mod page_flip;

use std::rc::Rc;

use anyhow::{Context as _, Result};
use drm_device::DpmsValue;
use glutin::{
    context::{ContextAttributesBuilder, NotCurrentContext, Priority},
    display::GetGlDisplay,
    prelude::GlDisplay,
};

use self::{drm_device::DrmDevice, gbm_data::GbmData, page_flip::PageFlipper};
use super::ApplicationContext;
use crate::gl::FutureGlThreadContext;

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

pub fn start_gbm<T>() -> Result<()>
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

    let mut page_flipper =
        PageFlipper::init(&gbm_data.device, &surface).context("Cannot create page flipper")?;

    let mut app = T::new(Rc::clone(&gl), bg_gl).context("Cannot create application")?;
    loop {
        let result = app.draw_frame().context("Error while drawing a frame")?;

        match result {
            super::DrawResult::FrameDrawn => page_flipper.flip()?,
            super::DrawResult::TurnDisplayOff => {
                gbm_data
                    .device
                    .set_dpms_property(DpmsValue::Off)
                    .context("Cannot turn off display")?;
            }
            super::DrawResult::TurnDisplayOn => {
                gbm_data
                    .device
                    .set_dpms_property(DpmsValue::On)
                    .context("Cannot turn on display")?;
            }
        }
    }
}
