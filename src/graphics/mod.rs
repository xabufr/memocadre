mod blur;
mod epaint_display;
mod image_display;

use std::{f32::consts::PI, ops::Deref, rc::Rc};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use epaint::Shape;
use image::DynamicImage;
use vek::{Extent2, FrustumPlanes, Mat4};

use self::epaint_display::EpaintDisplay;
#[cfg(test)]
pub use self::image_display::TextureRegion;
pub use self::{
    blur::ImageBlurr,
    epaint_display::{ShapeContainer, TextContainer},
    image_display::{ImageDrawer, Sprite},
};
use crate::{
    configuration::OrientationName,
    gl::{
        texture::{DetachedTexture, Texture},
        GlContext,
    },
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct SharedTexture2d(Rc<Texture>);

impl PartialEq for SharedTexture2d {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl SharedTexture2d {
    pub fn new(texture: Texture) -> Self {
        Self(Rc::new(texture))
    }
}

impl Deref for SharedTexture2d {
    type Target = Texture;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    image_drawer: ImageDrawer,
    blurr: ImageBlurr,
    epaint_display: EpaintDisplay,
    view: Mat4<f32>,
    orientation: Orientation,
    dimensions: Extent2<u32>,
    gl: Rc<GlContext>,
}

pub trait Drawable {
    fn draw(&self, graphics: &Graphics) -> Result<()>;
}

impl Graphics {
    pub fn new(gl: Rc<GlContext>, orientation: OrientationName) -> Result<Self> {
        let image_drawer = ImageDrawer::new(Rc::clone(&gl)).context("Cannot create ImageDrawer")?;
        let blurr = ImageBlurr::new(Rc::clone(&gl)).context("Cannot create ImageBlurr")?;
        let epaint_display =
            EpaintDisplay::new(Rc::clone(&gl)).context("Cannot create EpaintDisplay")?;

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

    #[allow(dead_code)]
    pub fn texture_from_image(&self, image: &DynamicImage) -> Result<Texture> {
        Texture::new_from_image(Rc::clone(&self.gl), image)
    }

    #[allow(dead_code)]
    pub fn blurr(&self) -> &ImageBlurr {
        &self.blurr
    }

    pub fn texture_from_detached(&self, detached: DetachedTexture) -> Texture {
        Texture::from_detached(Rc::clone(&self.gl), detached)
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

    pub fn create_text_container(&mut self) -> Result<TextContainer> {
        self.epaint_display.create_text_container()
    }

    pub fn force_text_container_update(&mut self, container: &TextContainer) {
        container.force_update(&mut self.epaint_display);
    }

    #[allow(dead_code)]
    pub fn create_shape(
        &mut self,
        shape: Shape,
        texture: Option<SharedTexture2d>,
    ) -> Result<ShapeContainer> {
        self.epaint_display.create_shape(shape, texture)
    }

    fn update_vp(&mut self) {
        // TODO: better way to get dims?
        let vp = self.gl.current_viewport();
        let mut dimensions = vp.extent().as_::<u32>();
        match self.orientation.name {
            OrientationName::Angle0 | OrientationName::Angle180 => {}
            OrientationName::Angle90 | OrientationName::Angle270 => {
                dimensions.swap(0, 1);
            }
        }
        if dimensions == self.dimensions {
            return;
        }
        self.dimensions = dimensions;
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

    fn view(&self) -> Mat4<f32> {
        self.view
    }

    fn image_drawer(&self) -> &ImageDrawer {
        &self.image_drawer
    }

    fn epaint_display(&self) -> &EpaintDisplay {
        &self.epaint_display
    }
}
