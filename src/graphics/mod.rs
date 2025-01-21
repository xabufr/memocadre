use bytemuck::{Pod, Zeroable};
use std::rc::Rc;

pub use blur::GlowImageBlurr;
pub use epaint_display::EpaintDisplay;
pub use image_display::{GlowImageDrawer, Sprite};

use crate::gl::Texture;

mod blur;
mod epaint_display;
mod image_display;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub type SharedTexture2d = Rc<Texture>;
