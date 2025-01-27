use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use epaint::Shape;
use epaint_display::{ShapeContainer, TextContainer};
use std::rc::Rc;
use vek::{FrustumPlanes, Mat4};

pub use blur::{BlurOptions, ImageBlurr};
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
    view: Mat4<f32>,
    gl: GlContext,
}

pub trait Drawable {
    fn draw(&self, graphics: &Graphics) -> Result<()>;
}

impl Graphics {
    pub fn new(gl: GlContext) -> Result<Self> {
        let image_drawer =
            ImageDrawert::new(GlContext::clone(&gl)).context("Cannot create ImageDrawer")?;
        let blurr = ImageBlurr::new(GlContext::clone(&gl)).context("Cannot create ImageBlurr")?;
        let epaint_display =
            EpaintDisplay::new(GlContext::clone(&gl)).context("Cannot create EpaintDisplay")?;

        Ok(Self {
            image_drawer,
            blurr,
            epaint_display,
            gl,
            view: Mat4::zero(),
        })
    }

    pub fn begin_frame(&mut self) {
        self.epaint_display.begin_frame();
    }

    pub fn update(&mut self) {
        self.epaint_display.update();

        // TODO better way to get dims?
        let vp = self.gl.current_viewport();
        self.view = Mat4::orthographic_without_depth_planes(FrustumPlanes {
            left: 0.,
            right: vp.w as _,
            bottom: vp.h as _,
            top: 0.,
            far: -1.,
            near: 1.,
        });
    }

    pub fn draw<D: Drawable>(&self, drawable: &D) -> Result<()> {
        drawable.draw(self)
    }

    pub fn create_text_container(&mut self) -> TextContainer {
        self.epaint_display.create_text_container()
    }

    pub fn force_text_container_update(&mut self, container: &TextContainer) {
        self.epaint_display.force_container_update(container);
    }

    pub fn create_shape(
        &mut self,
        shape: Shape,
        texture: Option<SharedTexture2d>,
    ) -> ShapeContainer {
        self.epaint_display.create_shape(shape, texture)
    }

    pub fn blurr(&self) -> &ImageBlurr {
        &self.blurr
    }
}

impl Drawable for Sprite {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics
            .image_drawer
            .draw_sprite(graphics.view, self)
            .context("Cannot draw sprite using ImageDrawer")
    }
}

impl Drawable for TextContainer {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics
            .epaint_display
            .draw_container(graphics.view, self)
            .context("Cannot draw text using epaint")
    }
}

impl Drawable for ShapeContainer {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics
            .epaint_display
            .draw_shape(graphics.view, self)
            .context("Cannot draw shape using epaint")
    }
}
