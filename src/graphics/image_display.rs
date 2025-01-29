use anyhow::{Context, Result};
use vek::{Extent2, Mat4, Rect, Vec2};

use super::{SharedTexture2d, Vertex2dUv};
use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    vao::{BufferInfo, VertexArrayObject},
    BlendMode, DrawParameters, GlContext, Program,
};

pub struct ImageDrawert {
    // vertex_array: glow::NativeVertexArray,
    // index_buffer: ElementBufferObject,
    // vertex_buffer: BufferObject<Vertex2dUv>,
    vao: VertexArrayObject<Vertex2dUv>,
    // index_buffer: glow::NativeBuffer,
    program: Program,
    gl: GlContext,
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
    //
    pub scissor: Option<Rect<i32, i32>>,
}

impl Sprite {
    pub fn new(texture: SharedTexture2d) -> Self {
        Self {
            position: Vec2::zero(),
            size: texture.size().as_(),
            opacity: 1.,
            texture,
            scissor: None,
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
}

#[rustfmt::skip]
const VERTICES: [Vertex2dUv; 4] = [
    Vertex2dUv { pos : [ 0., 0. ], uv: [ 0., 0. ] },
    Vertex2dUv { pos : [ 1., 0. ], uv: [ 1., 0. ] },
    Vertex2dUv { pos : [ 1., 1. ], uv: [ 1., 1. ] },
    Vertex2dUv { pos : [ 0., 1. ], uv: [ 0., 1. ] },
];
const INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

impl ImageDrawert {
    pub fn new(gl: GlContext) -> Result<Self> {
        let mut vbo =
            BufferObject::new_vertex_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw)
                .context("Cannot create VertexArray")?;
        let mut ebo =
            ElementBufferObject::new_index_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw)
                .context("Cannot create ElementBufferArray")?;

        let program = Program::new(GlContext::clone(&gl), shader::VERTEX, shader::FRAGMENT)
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
        let vao = VertexArrayObject::new(GlContext::clone(&gl), vbo, ebo, buffer_infos)
            .context("Cannot create ImageDrawer VAO")?;
        Ok(Self { vao, program, gl })
    }

    pub fn draw_sprite(&self, view: Mat4<f32>, sprite: &Sprite) -> Result<()> {
        let model = Mat4::scaling_3d(Vec2::from(sprite.size)).translated_2d(sprite.position);

        let prog_bind = self.program.bind();

        prog_bind.set_uniform("opacity", sprite.opacity)?;
        prog_bind.set_uniform("model", model)?;
        prog_bind.set_uniform("view", view)?;
        prog_bind.set_uniform("tex", 0)?;

        sprite.texture.bind(Some(0));

        let _guard = self.vao.bind_guard();

        self.gl.draw(
            &_guard,
            &prog_bind,
            INDICES.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
                scissor: sprite.scissor,
            },
        );
        Ok(())
    }
}

mod shader {
    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv;

    uniform mat4 model;
    uniform mat4 view;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = view * model * vec4(pos, 0, 1);
        texcoord = uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;
    uniform lowp float opacity;

    void main() {
        gl_FragColor = vec4(texture2D(tex, texcoord).rgb, opacity);
    }"#;
}
