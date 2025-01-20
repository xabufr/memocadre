use bytemuck::{Pod, Zeroable};
use glam::UVec2;
use glow::HasContext;
use image::{DynamicImage, GenericImageView};
use std::rc::Rc;

use super::GlContext;

pub type SharedTexture2d = Rc<Texture>;
pub struct Texture {
    texture: glow::Texture,
    size: UVec2,
}

impl Texture {
    pub fn new_from_image(gl: &GlContext, image: &DynamicImage) -> Self {
        Self {
            size: image.dimensions().into(),
            texture: unsafe { Self::load_texture(gl, image) },
        }
    }

    pub fn empty(
        gl: &GlContext,
        internal_format: i32,
        dimensions: UVec2,
        format: u32,
        ty: u32,
    ) -> Self {
        let mut tex = unsafe {
            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as _,
                dimensions.x as _,
                dimensions.y as _,
                0,
                glow::RGB,
                ty,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as _);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as _);
            gl.bind_texture(glow::TEXTURE_2D, None);
            Self {
                size: dimensions,
                texture,
            }
        };
        tex.reset(gl, internal_format, dimensions, format, ty);
        return tex;
    }

    pub fn reset(
        &mut self,
        gl: &GlContext,
        internal_format: i32,
        dimensions: UVec2,
        format: u32,
        ty: u32,
    ) {
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format,
                dimensions.x as _,
                dimensions.y as _,
                0,
                format,
                ty,
                glow::PixelUnpackData::Slice(None),
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        self.size = dimensions;
    }

    pub fn get(&self) -> glow::Texture {
        return self.texture;
    }

    pub fn size(&self) -> UVec2 {
        return self.size;
    }

    pub fn max_texture_size(gl: &glow::Context) -> u32 {
        return unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) } as u32;
    }

    unsafe fn load_texture(gl: &glow::Context, image: &DynamicImage) -> glow::Texture {
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        let image_data = image.to_rgb8().into_raw();
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as _,
            image.width() as i32,
            image.height() as i32,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(image_data.as_slice())),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as _,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as _);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as _);
        // gl.generate_mipmap(glow::TEXTURE_2D);
        gl.bind_texture(glow::TEXTURE_2D, None);
        texture
    }
}
