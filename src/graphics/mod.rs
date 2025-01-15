use glium::Texture2d;
use std::rc::Rc;

pub use blur::ImageBlurr;
pub use epaint::EpaintDisplay;
pub use image_display::{ImageDrawer, Sprite};
pub use text_display::TextDisplay;

mod blur;
mod epaint;
mod image_display;
mod text_display;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);

pub type SharedTexture2d = Rc<Texture2d>;
