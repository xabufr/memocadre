mod drm_device;
mod gbm_data;
mod page_flip;

use std::{rc::Rc, sync::Arc, thread::sleep, time::Duration};

use anyhow::{Context as _, Result};
use drm_device::DpmsValue;
use glutin::{
    context::{ContextAttributesBuilder, NotCurrentContext, Priority},
    display::GetGlDisplay,
    prelude::GlDisplay,
};

use self::{drm_device::DrmDevice, gbm_data::GbmData, page_flip::PageFlipper};
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

    let mut page_flipper = PageFlipper::new(&gbm_data.device, &surface);
    let (mut bo, mut fb) = page_flipper.initial_flip()?;
    // Drop mutability
    let page_flipper = page_flipper;

    let mut app = T::new(app_config, Rc::clone(&gl), bg_gl).context("Cannot create application")?;
    // let enabled = false;
    loop {
        // // TODO implement switching
        // if enabled {
        app.draw_frame().context("Error while drawing a frame")?;

        page_flipper.flip(&mut bo, &mut fb)?;
        // } else {
        //     gbm_data.device.set_dpms_property(DpmsValue::Standby)?;
        //     sleep(Duration::from_secs(60));
        // }
    }
}
