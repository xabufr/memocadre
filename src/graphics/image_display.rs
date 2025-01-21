use glam::{Mat4, Quat, Vec2, Vec3};

use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    vao::{BufferInfo, VertexArrayObject},
    BlendMode, DrawParameters, GlContext, Program,
};

use super::{SharedTexture2d, Vertex2dUv};

pub struct GlowImageDrawer {
    // vertex_array: glow::NativeVertexArray,
    // index_buffer: ElementBufferObject,
    // vertex_buffer: BufferObject<Vertex2dUv>,
    vao: VertexArrayObject<Vertex2dUv>,
    // index_buffer: glow::NativeBuffer,
    program: Program,
}

pub struct Sprite {
    pub texture: SharedTexture2d,
    // Position of the sprite in pixels on the screen
    // By default, this is (0, 0)
    pub position: Vec2,
    // Size of the sprite in pixels
    // By default, this is the size of the texture
    pub size: Vec2,
    //
    pub opacity: f32,
}

impl Sprite {
    pub fn new(texture: SharedTexture2d) -> Self {
        Self {
            position: Vec2::ZERO,
            size: texture.size().as_vec2(),
            opacity: 1.,
            texture,
        }
    }

    // Scales the sprite to fit the given dimensions while maintaining aspect ratio
    pub fn resize_respecting_ratio(&mut self, target_size: Vec2) {
        let tex_size = self.get_texture_size();
        let ratio = target_size / tex_size;
        let ratio = ratio.min_element();
        self.size = tex_size * ratio;
    }

    pub fn get_texture_size(&self) -> Vec2 {
        self.texture.size().as_vec2()
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

impl GlowImageDrawer {
    pub fn new(gl: &GlContext) -> Self {
        let vbo = BufferObject::new_vertex_buffer(GlContext::clone(gl), BufferUsage::StaticDraw);
        let ebo =
            ElementBufferObject::new_index_buffer(GlContext::clone(gl), BufferUsage::StaticDraw);

        let program = Program::new(GlContext::clone(gl), shader::VERTEX, shader::FRAGMENT);
        let pos = program.get_attrib_location("pos");
        let uv = program.get_attrib_location("uv");

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
        let vao = VertexArrayObject::new(GlContext::clone(gl), vbo, ebo, buffer_infos);
        Self { vao, program }
    }
    pub fn draw_sprite(&self, gl: &GlContext, sprite: &Sprite) {
        let model = Mat4::from_scale_rotation_translation(
            Vec3::from((sprite.size, 0.)),
            Quat::IDENTITY,
            Vec3::from((sprite.position, 0.)),
        );

        let (_, _, width, height) = gl.current_viewport();

        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let prog_bind = self.program.bind();

        prog_bind.set_uniform("opacity", sprite.opacity);
        prog_bind.set_uniform("model", model);
        prog_bind.set_uniform("view", view);
        prog_bind.set_uniform("tex", 0);

        sprite.texture.bind(Some(0));

        let _guard = self.vao.bind_guard();

        gl.draw(
            &_guard,
            &prog_bind,
            INDICES.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
            },
        );
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
