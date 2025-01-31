use anyhow::{Error, Result};
use glow::HasContext;
use image::{DynamicImage, GenericImageView};
use vek::{Extent2, Rect};

use super::GlContext;

pub struct Texture {
    texture: glow::Texture,
    size: Extent2<u32>,
    format: TextureFormat,
    options: TextureOptions,
    gl: GlContext,
}

#[derive(Copy, Clone, Default)]
pub struct TextureOptions {
    pub mag: TextureFiltering,
    pub min: TextureFiltering,
    pub wrap: TextureWrapMode,
}

#[derive(Copy, Clone)]
pub enum TextureFormat {
    Rgba,
    Rgb,
}
#[derive(Copy, Clone)]
pub enum TextureFiltering {
    Nearest,
    Linear,
}
#[derive(Copy, Clone)]
pub enum TextureWrapMode {
    ClampToEdge,
    MirroredRepeat,
    Repeat,
}
impl Default for TextureWrapMode {
    fn default() -> Self {
        Self::Repeat
    }
}
impl Default for TextureFiltering {
    fn default() -> Self {
        Self::Linear
    }
}
impl TextureFormat {
    fn to_gl(self) -> u32 {
        match self {
            TextureFormat::Rgba => glow::RGBA,
            TextureFormat::Rgb => glow::RGB,
        }
    }

    fn bytes_per_pixel(self) -> usize {
        match self {
            TextureFormat::Rgba => 4,
            TextureFormat::Rgb => 3,
        }
    }
}
impl TextureFiltering {
    fn to_gl(self) -> i32 {
        match self {
            TextureFiltering::Nearest => glow::NEAREST as _,
            TextureFiltering::Linear => glow::LINEAR as _,
        }
    }
}
impl TextureWrapMode {
    fn to_gl(self) -> i32 {
        (match self {
            TextureWrapMode::ClampToEdge => glow::CLAMP_TO_EDGE,
            TextureWrapMode::MirroredRepeat => glow::MIRRORED_REPEAT,
            TextureWrapMode::Repeat => glow::REPEAT,
        }) as i32
    }
}

const TARGET: u32 = glow::TEXTURE_2D;

impl Texture {
    pub fn new_from_image(gl: GlContext, image: &DynamicImage) -> Result<Self> {
        let mut tex = Self {
            size: image.dimensions().into(),
            texture: unsafe { Self::load_texture(&gl, image)? },
            format: TextureFormat::Rgb,
            options: Default::default(),
            gl,
        };
        tex.set_options(Default::default());
        Ok(tex)
    }

    pub fn empty(gl: GlContext, format: TextureFormat, dimensions: Extent2<u32>) -> Result<Self> {
        let mut tex = unsafe {
            let texture = gl.create_texture().map_err(Error::msg)?;
            gl.bind_texture(TARGET, Some(texture));
            gl.tex_image_2d(
                TARGET,
                0,
                format.to_gl() as _,
                dimensions.w as _,
                dimensions.h as _,
                0,
                format.to_gl(),
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.bind_texture(TARGET, None);
            Self {
                size: dimensions,
                gl,
                format,
                texture,
                options: Default::default(),
            }
        };
        tex.set_options(Default::default());
        Ok(tex)
    }

    pub fn set_options(&mut self, options: TextureOptions) {
        self.options = options;
        unsafe {
            self.gl.bind_texture(TARGET, Some(self.texture));
            self.gl
                .tex_parameter_i32(TARGET, glow::TEXTURE_MIN_FILTER, options.min.to_gl());
            self.gl
                .tex_parameter_i32(TARGET, glow::TEXTURE_MAG_FILTER, options.mag.to_gl());
            self.gl
                .tex_parameter_i32(TARGET, glow::TEXTURE_WRAP_S, options.wrap.to_gl());
            self.gl
                .tex_parameter_i32(TARGET, glow::TEXTURE_WRAP_T, options.wrap.to_gl());
            self.gl.bind_texture(TARGET, None);
        }
    }

    pub fn write(&mut self, format: TextureFormat, dimensions: Extent2<u32>, data: &[u8]) {
        assert_eq!(
            (dimensions.w * dimensions.h) as usize * format.bytes_per_pixel(),
            data.len()
        );
        unsafe {
            self.gl.bind_texture(TARGET, Some(self.texture));
            self.gl.tex_image_2d(
                TARGET,
                0,
                format.to_gl() as _,
                dimensions.w as _,
                dimensions.h as _,
                0,
                format.to_gl(),
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            self.gl.bind_texture(TARGET, None);
        }
        self.format = format;
        self.size = dimensions;
    }

    pub fn write_sub(&self, region: Rect<u32, u32>, data: &[u8]) {
        assert_eq!(
            (region.w * region.h) as usize * self.format.bytes_per_pixel(),
            data.len()
        );
        unsafe {
            self.gl.bind_texture(TARGET, Some(self.texture));
            self.gl.tex_sub_image_2d(
                TARGET,
                0,
                region.x as _,
                region.y as _,
                region.w as _,
                region.h as _,
                self.format.to_gl(),
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(data)),
            );
            self.gl.bind_texture(TARGET, None);
        }
    }

    pub fn get(&self) -> glow::Texture {
        self.texture
    }

    pub fn size(&self) -> Extent2<u32> {
        self.size
    }

    unsafe fn load_texture(gl: &glow::Context, image: &DynamicImage) -> Result<glow::Texture> {
        let texture = gl.create_texture().map_err(Error::msg)?;
        gl.bind_texture(TARGET, Some(texture));
        // FIXME set in graphics init
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        let image_data = image.to_rgb8().into_raw();
        gl.tex_image_2d(
            TARGET,
            0,
            glow::RGB as _,
            image.width() as i32,
            image.height() as i32,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(image_data.as_slice())),
        );
        gl.bind_texture(TARGET, None);
        Ok(texture)
    }

    pub fn bind(&self, channel: Option<u8>) {
        unsafe {
            if let Some(channel) = channel {
                self.gl.active_texture(glow::TEXTURE0 + channel as u32);
            }
            self.gl.bind_texture(TARGET, Some(self.texture));
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { self.gl.delete_texture(self.texture) };
    }
}
