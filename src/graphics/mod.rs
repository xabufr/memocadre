use std::rc::Rc;

pub use blur::ImageBlurr;
use glium::Texture2d;
pub use image_display::{ImageDrawer, Sprite};

mod blur;
mod image_display;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);

pub type SharedTexture2d = Rc<Texture2d>;
