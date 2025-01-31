mod blur;
mod epaint_display;
mod image_display;

use std::{f32::consts::PI, rc::Rc};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use epaint::Shape;
use image::DynamicImage;
use vek::{Extent2, FrustumPlanes, Mat4};

use self::epaint_display::EpaintDisplay;
pub use self::{
    blur::{BlurOptions, ImageBlurr},
    epaint_display::{ShapeContainer, TextContainer},
    image_display::{ImageDrawert, Sprite},
};
use crate::{
    configuration::OrientationName,
    gl::{GlContext, Texture},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub type SharedTexture2d = Rc<Texture>;

struct Orientation {
    name: OrientationName,
    value: Mat4<f32>,
}

impl Orientation {
    fn create(name: OrientationName) -> Self {
        Self {
            name,
            value: name.get_mat(),
        }
    }
}

impl OrientationName {
    // TODO: make this a const fn
    fn get_mat(&self) -> Mat4<f32> {
        match self {
            Self::Angle0 => Mat4::identity(),
            Self::Angle90 => Mat4::rotation_z(PI * 0.5),
            Self::Angle180 => Mat4::rotation_z(PI),
            Self::Angle270 => Mat4::rotation_z(PI * 1.5),
        }
    }
}

pub struct Graphics {
    image_drawer: ImageDrawert,
    blurr: ImageBlurr,
    epaint_display: EpaintDisplay,
    view: Mat4<f32>,
    orientation: Orientation,
    dimensions: Extent2<u32>,
    gl: GlContext,
}

pub trait Drawable {
    fn draw(&self, graphics: &Graphics) -> Result<()>;
}

impl Graphics {
    pub fn new(gl: GlContext, orientation: OrientationName) -> Result<Self> {
        let image_drawer =
            ImageDrawert::new(GlContext::clone(&gl)).context("Cannot create ImageDrawer")?;
        let blurr = ImageBlurr::new(GlContext::clone(&gl)).context("Cannot create ImageBlurr")?;
        let epaint_display =
            EpaintDisplay::new(GlContext::clone(&gl)).context("Cannot create EpaintDisplay")?;

        let mut graphics = Self {
            image_drawer,
            blurr,
            epaint_display,
            gl,
            orientation: Orientation::create(orientation),
            dimensions: Extent2::default(),
            view: Mat4::zero(),
        };
        graphics.update_vp();
        Ok(graphics)
    }

    pub fn texture_from_image(&self, image: &DynamicImage) -> Result<Texture> {
        Texture::new_from_image(GlContext::clone(&self.gl), image)
    }

    pub fn begin_frame(&mut self) {
        self.epaint_display.begin_frame();

        self.update_vp();
    }

    pub fn update(&mut self) {
        self.epaint_display.update();
    }

    pub fn get_dimensions(&self) -> Extent2<u32> {
        self.dimensions
    }

    pub fn draw<D: Drawable>(&self, drawable: &D) -> Result<()> {
        drawable.draw(self)
    }

    pub fn create_text_container(&mut self) -> Result<TextContainer> {
        self.epaint_display.create_text_container()
    }

    pub fn force_text_container_update(&mut self, container: &TextContainer) {
        self.epaint_display.force_container_update(container);
    }

    #[allow(dead_code)]
    pub fn create_shape(
        &mut self,
        shape: Shape,
        texture: Option<SharedTexture2d>,
    ) -> Result<ShapeContainer> {
        self.epaint_display.create_shape(shape, texture)
    }

    pub fn blurr(&self) -> &ImageBlurr {
        &self.blurr
    }

    fn update_vp(&mut self) {
        // TODO: better way to get dims?
        let vp = self.gl.current_viewport();
        self.dimensions = vp.extent().as_();
        match self.orientation.name {
            OrientationName::Angle0 | OrientationName::Angle180 => {}
            OrientationName::Angle90 | OrientationName::Angle270 => {
                self.dimensions.swap(0, 1);
            }
        }
        self.view = self.orientation.value
            * Mat4::orthographic_without_depth_planes(FrustumPlanes {
                left: 0.,
                right: self.dimensions.w as _,
                bottom: self.dimensions.h as _,
                top: 0.,
                far: -1.,
                near: 1.,
            });
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
