use std::sync::Arc;

use anyhow::Result;

mod gbm_display;
mod window_display;

pub use gbm_display::start_gbm;
pub use window_display::State;

use crate::{configuration::Conf, gl::GlContext};

pub trait ApplicationContext: Sized {
    fn draw_frame(&mut self) -> Result<()> {
        Ok(())
    }
    fn new(config: Arc<Conf>, gl: GlContext) -> Result<Self>;
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
