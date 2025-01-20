use glam::{Mat4, Quat, Vec2, Vec3};
use glow::HasContext;

use super::{SharedTexture2d, Vertex2dUv};

pub struct GlowImageDrawer {
    vertex_array: glow::NativeVertexArray,
    vertex_buffer: glow::NativeBuffer,
    index_buffer: glow::NativeBuffer,
    program: super::Program,
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
const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl GlowImageDrawer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();

            let program = crate::graphics::Program::new(gl, shader::VERTEX, shader::FRAGMENT);
            let pos = gl.get_attrib_location(program.get(), "pos").unwrap();
            let uv = gl.get_attrib_location(program.get(), "uv").unwrap();

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&VERTICES),
                glow::STATIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&INDICES),
                glow::STATIC_DRAW,
            );

            gl.enable_vertex_attrib_array(pos);
            gl.enable_vertex_attrib_array(uv);
            gl.vertex_attrib_pointer_f32(
                pos,
                2,
                glow::FLOAT,
                false,
                std::mem::size_of::<Vertex2dUv>() as i32,
                memoffset::offset_of!(Vertex2dUv, pos) as i32,
            );
            gl.vertex_attrib_pointer_f32(
                uv,
                2,
                glow::FLOAT,
                false,
                std::mem::size_of::<Vertex2dUv>() as i32,
                memoffset::offset_of!(Vertex2dUv, uv) as i32,
            );

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            Self {
                vertex_array: vao,
                vertex_buffer: vbo,
                index_buffer: ebo,
                program,
            }
        }
    }
    pub fn draw_sprite(&self, gl: &glow::Context, sprite: &Sprite) {
        unsafe {
            let model = Mat4::from_scale_rotation_translation(
                Vec3::from((sprite.size, 0.)),
                Quat::IDENTITY,
                Vec3::from((sprite.position, 0.)),
            );

            let mut dims: [i32; 4] = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut dims);
            let [_, _, width, height] = dims;

            let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
            gl.use_program(Some(self.program.get()));

            let opacity = gl.get_uniform_location(self.program.get(), "opacity");
            gl.uniform_1_f32(opacity.as_ref(), sprite.opacity);
            let model_position = gl.get_uniform_location(self.program.get(), "model");
            gl.uniform_matrix_4_f32_slice(model_position.as_ref(), false, &model.to_cols_array());
            let view_position = gl.get_uniform_location(self.program.get(), "view");
            gl.uniform_matrix_4_f32_slice(view_position.as_ref(), false, &view.to_cols_array());

            let tex_position = gl.get_uniform_location(self.program.get(), "tex");
            gl.uniform_1_i32(tex_position.as_ref(), 0);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(sprite.texture.get()));

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_elements(glow::TRIANGLES, INDICES.len() as _, glow::UNSIGNED_SHORT, 0);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
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
