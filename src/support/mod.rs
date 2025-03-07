#[cfg(feature = "drm")]
mod gbm_display;
#[cfg(feature = "winit")]
mod window_display;

use std::rc::Rc;

use anyhow::{Context, Result};
use tokio::sync::watch;

#[cfg(feature = "drm")]
use self::gbm_display::start_gbm;
#[cfg(feature = "winit")]
use self::window_display::State;
use crate::{
    configuration::AppConfiguration,
    gl::{FutureGlThreadContext, GlContext},
};

pub trait ApplicationContext: Sized {
    fn draw_frame(&mut self) -> Result<()> {
        Ok(())
    }
    fn new(
        config: watch::Sender<AppConfiguration>,
        gl: Rc<GlContext>,
        bg_gl: FutureGlThreadContext,
    ) -> Result<Self>;
    #[cfg(feature = "winit")]
    fn resized(&mut self, _width: u32, _height: u32) {}
    #[cfg(feature = "winit")]
    fn handle_window_event(
        &mut self,
        _event: &winit::event::WindowEvent,
        _window: &winit::window::Window,
    ) {
    }
    const WINDOW_TITLE: &'static str;
}

pub fn start<T: ApplicationContext + 'static>(config: AppConfiguration) -> Result<()> {
    let config = watch::Sender::new(config);

    #[cfg(feature = "winit")]
    {
        let vars = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"];
        let has_window_system = vars.into_iter().any(|v| std::env::var_os(v).is_some());
        if has_window_system {
            return State::<T>::run_loop(config).context("While running application");
        }
    }
    #[cfg(feature = "drm")]
    {
        #[allow(clippy::needless_return)]
        return start_gbm::<T>(config).context("While running application");
    }

    #[cfg(not(feature = "drm"))]
    return Err(anyhow::anyhow!("No window system available"));
}
