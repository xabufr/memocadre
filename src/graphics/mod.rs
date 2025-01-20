use bytemuck::{Pod, Zeroable};
use glium::Texture2d;
use std::rc::Rc;

pub use blur::GlowImageBlurr;
// pub use epaint_display::EpaintDisplay;
pub use image_display::{GlowImageDrawer, Sprite};
// pub use text_display::TextDisplay;
pub use shader::Program;
pub use texture::{SharedTexture2d, Texture};

mod blur;
// mod epaint_display;
mod image_display;
mod shader;
mod texture;
// mod text_display;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);
