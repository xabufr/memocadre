use bytemuck::{Pod, Zeroable};
use glium::draw_parameters;
use glow::HasContext;
use std::{cell::RefCell, ops::Deref, rc::Rc};

pub use blur::GlowImageBlurr;
pub use epaint_display::EpaintDisplay;
pub use image_display::{GlowImageDrawer, Sprite};

use crate::gl::Texture;
// pub use text_display::TextDisplay;

mod blur;
mod image_display;
// pub mod pipeline;
mod epaint_display;
// mod text_display;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);

pub type SharedTexture2d = Rc<Texture>;
