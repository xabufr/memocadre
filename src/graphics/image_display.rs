use std::rc::Rc;

use anyhow::{Context, Result};
use vek::{num_traits::Inv, Extent2, Mat4, Rect, Vec2};

use super::{Drawable, Graphics, SharedTexture2d, Vertex2dUv};
use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    shader::{Program, ProgramGuard},
    vao::{BufferInfo, VertexArrayObject},
    BlendMode, DrawParameters, GlContext,
};

pub struct ImageDrawer {
    // vertex_array: glow::NativeVertexArray,
    // index_buffer: ElementBufferObject,
    // vertex_buffer: BufferObject<Vertex2dUv>,
    vao: VertexArrayObject<Vertex2dUv>,
    // index_buffer: glow::NativeBuffer,
    program: Program,
    gl: Rc<GlContext>,
}

#[derive(Debug, Clone, Copy)]
pub struct TextureRegion {
    pub uv_center: Vec2<f32>,
    pub uv_size: Extent2<f32>,
}

pub struct Sprite {
    pub texture: SharedTexture2d,
    // Position of the sprite in pixels on the screen
    // By default, this is (0, 0)
    pub position: Vec2<f32>,
    // Size of the sprite in pixels
    // By default, this is the size of the texture
    pub size: Extent2<f32>,
    //
    pub opacity: f32,

    sub_rect: TextureRegion,
}

const DEFAULT_SUB_RECT: TextureRegion = TextureRegion {
    uv_center: Vec2::new(0.5, 0.5),
    uv_size: Extent2::new(0.5, 0.5),
};

impl Sprite {
    pub fn new(texture: SharedTexture2d) -> Self {
        Self {
            position: Vec2::zero(),
            size: texture.size().as_(),
            opacity: 1.,
            texture,
            sub_rect: DEFAULT_SUB_RECT,
        }
    }

    // Scales the sprite to fit the given dimensions while maintaining aspect ratio
    pub fn resize_respecting_ratio(&mut self, target_size: Extent2<u32>) {
        let target_size: Extent2<f32> = target_size.as_();
        let tex_size: Extent2<f32> = self.get_texture_size().as_();
        let ratio = target_size / tex_size;
        let ratio = ratio.reduce_partial_min();
        self.size = tex_size * ratio;
    }

    pub fn get_texture_size(&self) -> Extent2<u32> {
        self.texture.size()
    }

    pub fn set_sub_rect(&mut self, sub_rect: Rect<i32, i32>) {
        let sub_rect = sub_rect.as_::<f32, f32>();
        let tr = Vec2::from(self.get_texture_size().as_::<f32>()).inv();
        self.sub_rect.uv_center = sub_rect.center() * tr;
        self.sub_rect.uv_size = sub_rect.extent() * tr * 0.5;
    }

    pub fn set_sub_center_size(&mut self, uv_offset_center: Vec2<f32>, uv_offset_size: Vec2<f32>) {
        self.sub_rect.uv_center = uv_offset_center;
        self.sub_rect.uv_size = uv_offset_size.into();
    }

    #[cfg(test)]
    pub fn get_sub_center_size(&self) -> TextureRegion {
        self.sub_rect
    }
}

impl Drawable for Sprite {
    #[inline]
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics
            .image_drawer()
            .draw_sprite(graphics.view(), self)
            .context("Cannot draw sprite using ImageDrawer")
    }
}

#[rustfmt::skip]
const VERTICES: [Vertex2dUv; 4] = [
    Vertex2dUv { pos : [ 0., 0. ], uv: [ 0., 0. ] },
    Vertex2dUv { pos : [ 1., 0. ], uv: [ 1., 0. ] },
    Vertex2dUv { pos : [ 1., 1. ], uv: [ 1., 1. ] },
    Vertex2dUv { pos : [ 0., 1. ], uv: [ 0., 1. ] },
];
const INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

impl ImageDrawer {
    pub fn new(gl: Rc<GlContext>) -> Result<Self> {
        let mut vbo = BufferObject::new_vertex_buffer(Rc::clone(&gl), BufferUsage::Static)
            .context("Cannot create VertexArray")?;
        let mut ebo = ElementBufferObject::new_index_buffer(Rc::clone(&gl), BufferUsage::Static)
            .context("Cannot create ElementBufferArray")?;

        let program = Program::new(Rc::clone(&gl), shader::VERTEX, shader::FRAGMENT)
            .context("Cannot create ImageDrawer shader")?;
        let pos = program.get_attrib_location("pos")?;
        let uv = program.get_attrib_location("uv")?;

        vbo.write(&VERTICES);
        ebo.write(&INDICES);

        let stride = std::mem::size_of::<Vertex2dUv>() as i32;
        let buffer_infos = vec![
            BufferInfo {
                location: pos,
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex2dUv, pos) as i32,
            },
            BufferInfo {
                location: uv,
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex2dUv, uv) as i32,
            },
        ];
        let vao = VertexArrayObject::new(Rc::clone(&gl), vbo, ebo, buffer_infos)
            .context("Cannot create ImageDrawer VAO")?;
        Ok(Self { vao, program, gl })
    }

    pub fn draw_sprite(&self, view: Mat4<f32>, sprite: &Sprite) -> Result<()> {
        let model = Mat4::scaling_3d(Vec2::from(sprite.size)).translated_2d(sprite.position);

        let prog_bind = ProgramGuard::bind(&self.program);

        prog_bind.set_uniform("opacity", sprite.opacity)?;
        prog_bind.set_uniform("model", model)?;
        prog_bind.set_uniform("view", view)?;
        prog_bind.set_uniform("tex", 0)?;
        prog_bind.set_uniform("uv_offset_center", sprite.sub_rect.uv_center)?;
        prog_bind.set_uniform("uv_offset_size", sprite.sub_rect.uv_size)?;

        sprite.texture.bind(Some(0));

        let _guard = self.vao.bind_guard();

        self.gl.draw(
            &_guard,
            &prog_bind,
            INDICES.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
            },
        );
        Ok(())
    }
}

mod shader {
    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv;

    uniform vec2 uv_offset_center;
    uniform vec2 uv_offset_size;
    uniform mat4 model;
    uniform mat4 view;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = view * model * vec4(pos, 0, 1);
        texcoord = (2. * uv - 1.) * uv_offset_size + uv_offset_center;
        }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;
    uniform lowp float opacity;

    void main() {
        gl_FragColor = vec4(texture2D(tex, texcoord).rgb, opacity);
    }"#;
}

#[cfg(test)]
mod test {
    use googletest::{expect_that, gtest, matchers::matches_pattern, prelude::approx_eq};
    use vek::Extent2;

    use super::*;
    use crate::gl::{texture::Texture, wrapper::mocked_gl};

    #[gtest]
    fn test_sprite_resize_respecting_ratio() {
        let gl = mocked_gl();
        let context = Rc::new(GlContext::mocked(gl));
        let texture = Texture::mocked(context.clone(), Extent2::new(100, 100));
        let mut sprite = Sprite::new(SharedTexture2d::new(texture));

        sprite.resize_respecting_ratio(Extent2::new(50, 50));
        expect_that!(
            sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(50.),
                h: approx_eq(50.)
            })
        );

        sprite.resize_respecting_ratio(Extent2::new(50, 40));
        expect_that!(
            sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(40.),
                h: approx_eq(40.)
            })
        );

        sprite.resize_respecting_ratio(Extent2::new(30, 40));
        expect_that!(
            sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(30.),
                h: approx_eq(30.)
            })
        );
    }

    #[gtest]
    fn test_sprite_set_sub_rect() {
        let gl = mocked_gl();
        let context = Rc::new(GlContext::mocked(gl));
        let texture = Texture::mocked(context.clone(), Extent2::new(100, 100));
        let mut sprite = Sprite::new(SharedTexture2d::new(texture));
        sprite.set_sub_rect(Rect::from((Vec2::new(10, 10), Extent2::new(10, 10))));
        expect_that!(
            sprite.sub_rect,
            matches_pattern!(TextureRegion {
                uv_center: matches_pattern!(Vec2 {
                    x: approx_eq(0.15),
                    y: approx_eq(0.15)
                }),
                uv_size: matches_pattern!(Extent2 {
                    w: approx_eq(0.05),
                    h: approx_eq(0.05)
                })
            })
        );
    }
}
