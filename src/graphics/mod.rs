use bytemuck::{Pod, Zeroable};
use std::rc::Rc;

pub use blur::ImageBlurr;
pub use epaint_display::EpaintDisplay;
pub use image_display::{ImageDrawert, Sprite};

use crate::gl::{GlContext, Texture};

mod blur;
pub mod epaint_display;
mod image_display;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub type SharedTexture2d = Rc<Texture>;

pub struct Graphics {
    image_drawer: ImageDrawert,
    blurr: ImageBlurr,
    epaint_display: EpaintDisplay,
}

impl Graphics {
    pub fn new(gl: GlContext) -> Self {
        let image_drawer = ImageDrawert::new(GlContext::clone(&gl));
        let blurr = ImageBlurr::new(GlContext::clone(&gl));
        let epaint_display = EpaintDisplay::new(GlContext::clone(&gl));

        Self {
            image_drawer,
            blurr,
            epaint_display,
        }
    }

    pub fn begin_frame(&mut self) {
        self.epaint_display.begin_frame();
    }

    pub fn update(&mut self) {
        self.epaint_display.update();
    }

    pub fn epaint(&self) -> &EpaintDisplay {
        &self.epaint_display
    }

    pub fn epaint_mut(&mut self) -> &mut EpaintDisplay {
        &mut self.epaint_display
    }

    pub fn image_drawer(&self) -> &ImageDrawert {
        &self.image_drawer
    }

    pub fn blurr(&self) -> &ImageBlurr {
        &self.blurr
    }
}
