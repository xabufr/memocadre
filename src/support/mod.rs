// mod gbm_display;
mod window_display;

use std::sync::Arc;

use anyhow::{Context, Result};

use self::{
    // gbm_display::start_gbm,
    window_display::State,
};
use crate::{
    configuration::Conf,
    gl::{FutureGlThreadContext, GlContext},
};

pub trait ApplicationContext: Sized {
    fn draw_frame(&mut self) -> Result<()> {
        Ok(())
    }
    fn new(config: Arc<Conf>, gl: GlContext, bg_gl: FutureGlThreadContext) -> Result<Self>;
    fn update(&mut self) {}
    fn resized(&mut self, _width: u32, _height: u32) {}
    fn handle_window_event(
        &mut self,
        _event: &winit::event::WindowEvent,
        _window: &winit::window::Window,
    ) {
    }
    const WINDOW_TITLE: &'static str;
}

pub fn start<T: ApplicationContext + 'static>(config: Conf) -> Result<()> {
    let vars = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"];
    let has_window_system = vars.into_iter().any(|v| std::env::var_os(v).is_some());
    let config = Arc::new(config);
    if has_window_system {
        State::<T>::run_loop(config)
    } else {
        // start_gbm::<T>(config)
        panic!("GBM not implemented yet")
    }
    .context("While running application")
}
