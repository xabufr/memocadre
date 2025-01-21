use bytemuck::{Pod, Zeroable};
use glam::UVec2;
use glow::HasContext;
use image::{DynamicImage, GenericImageView};

use super::GlContext;

pub struct Texture {
    texture: glow::Texture,
    size: UVec2,
    gl: GlContext,
}

pub enum TextureFormat {
    RGBA,
    RGB,
}
impl TextureFormat {
    fn to_gl(&self) -> u32 {
        match self {
            TextureFormat::RGBA => glow::RGBA,
            TextureFormat::RGB => glow::RGB,
        }
    }
}

impl Texture {
    pub fn new_from_image(gl: GlContext, image: &DynamicImage) -> Self {
        Self {
            size: image.dimensions().into(),
            texture: unsafe { Self::load_texture(&gl, image) },
            gl,
        }
    }

    pub fn empty(
        gl: GlContext,
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
                gl,
                texture,
            }
        };
        tex.reset(internal_format, dimensions, format, ty);
        return tex;
    }

    pub fn reset(&mut self, internal_format: i32, dimensions: UVec2, format: u32, ty: u32) {
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.tex_image_2d(
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
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
        self.size = dimensions;
    }

    pub fn write(&mut self, format: TextureFormat, dimensions: UVec2, data: &[u8]) {
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                format.to_gl() as _,
                dimensions.x as _,
                dimensions.y as _,
                0,
                format.to_gl(),
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );

            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as _,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as _,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
        self.size = dimensions;
    }

    pub fn write_sub(&self, format: TextureFormat, offset: UVec2, dimensions: UVec2, data: &[u8]) {
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                offset.x as _,
                offset.y as _,
                dimensions.x as _,
                dimensions.y as _,
                format.to_gl(),
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    pub fn get(&self) -> glow::Texture {
        return self.texture;
    }

    pub fn size(&self) -> UVec2 {
        return self.size;
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

    pub fn bind(&self, channel: Option<u8>) {
        unsafe {
            if let Some(channel) = channel {
                self.gl.active_texture(glow::TEXTURE0 + channel as u32);
            }
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { self.gl.delete_texture(self.texture) };
    }
}
