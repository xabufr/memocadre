use glium::Display;
use glutin::surface::WindowSurface;

mod gbm_display;
mod window_display;

pub use gbm_display::start_gbm;
pub use window_display::State;

pub trait ApplicationContext {
    fn draw_frame(&mut self, _display: &Display<WindowSurface>) {}
    fn new(display: &Display<WindowSurface>) -> Self;
    fn update(&mut self) {}
    fn handle_window_event(
        &mut self,
        _event: &winit::event::WindowEvent,
        _window: &winit::window::Window,
    ) {
    }
    const WINDOW_TITLE: &'static str;
}
