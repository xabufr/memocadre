use glow::HasContext;

use super::{Texture, Vertex2dUv};

pub struct GlowImageBlurr {
    vertex_array: glow::NativeVertexArray,
    vertex_buffer: glow::NativeBuffer,
    index_buffer: glow::NativeBuffer,
    program: super::Program,
}
// pub struct ImageBlurr {
//     vertex_buffer: VertexBuffer<Vertex2dUv>,
//     index_buffer: glium::IndexBuffer<u16>,
//     program: glium::Program,
// }

#[rustfmt::skip]
const VERTICES: [Vertex2dUv; 4] = [
    Vertex2dUv { pos : [ -1., -1. ], uv: [ 0., 0. ] },
    Vertex2dUv { pos : [  1., -1. ], uv: [ 1., 0. ] },
    Vertex2dUv { pos : [  1.,  1. ], uv: [ 1., 1. ] },
    Vertex2dUv { pos : [ -1.,  1. ], uv: [ 0., 1. ] },
];
const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
impl GlowImageBlurr {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();

            let program =
                crate::graphics::Program::new(gl, shader::VERTEX_BLUR, shader::FRAGMENT_BLUR);
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

    pub fn blur(&self, gl: &glow::Context, texture: &Texture) -> Texture {
        unsafe {
            let mut dims: [i32; 4] = [0; 4];
            unsafe {
                gl.get_parameter_i32_slice(glow::VIEWPORT, &mut dims);
            };
            let [_, _, width, height] = dims;
            let textures = [
                Texture::empty(
                    gl,
                    glow::RGB as _,
                    texture.size(),
                    glow::RGB,
                    glow::UNSIGNED_BYTE,
                ),
                Texture::empty(
                    gl,
                    glow::RGB as _,
                    texture.size(),
                    glow::RGB,
                    glow::UNSIGNED_BYTE,
                ),
            ];
            let fbos = [
                gl.create_framebuffer().unwrap(),
                gl.create_framebuffer().unwrap(),
            ];
            for (i, fbo) in fbos.iter().enumerate() {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(*fbo));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(textures[i].get()),
                    0,
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
            let mut source_texture = texture;

            let radius: f32 = 6.0;
            let passes = 6;
            let dir = self.program.get_uniform_location(gl, "dir").unwrap();
            gl.use_program(Some(self.program.get()));
            gl.bind_vertex_array(Some(self.vertex_array));
            let tex_size = self.program.get_uniform_location(gl, "tex_size").unwrap();
            gl.uniform_2_f32_slice(
                Some(&tex_size),
                texture.size().as_vec2().to_array().as_slice(),
            );
            let tex = self.program.get_uniform_location(gl, "tex").unwrap();
            gl.uniform_1_i32(Some(&tex), 0);
            gl.active_texture(glow::TEXTURE0);
            gl.viewport(0, 0, texture.size().x as _, texture.size().y as _);
            for i in 0..=passes {
                let radius = radius * (passes - i) as f32 / (passes as f32);

                gl.uniform_2_f32(Some(&dir), radius, 0.);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbos[0]));
                gl.bind_texture(glow::TEXTURE_2D, Some(source_texture.get()));
                gl.draw_elements(glow::TRIANGLES, INDICES.len() as _, glow::UNSIGNED_SHORT, 0);

                source_texture = &textures[0];

                gl.uniform_2_f32(Some(&dir), 0., radius);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbos[1]));
                gl.bind_texture(glow::TEXTURE_2D, Some(source_texture.get()));
                gl.draw_elements(glow::TRIANGLES, INDICES.len() as _, glow::UNSIGNED_SHORT, 0);

                source_texture = &textures[1];
            }
            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.use_program(None);
            gl.viewport(0, 0, width, height);
            for fbo in fbos {
                gl.delete_framebuffer(fbo);
            }

            let [_, texture] = textures;
            return texture;
        }
    }
}

mod shader {
    pub const VERTEX_BLUR: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(pos, 0, 1);
        texcoord = uv;
    }"#;
    pub const FRAGMENT_BLUR: &str = r#"#version 100
    precision mediump float;

    varying lowp vec2 texcoord;

    uniform sampler2D tex;
    uniform lowp vec2 tex_size;
    uniform lowp vec2 dir;

    vec4 blur5(sampler2D image, vec2 uv, vec2 resolution, vec2 direction) {
      vec4 color = vec4(0.0);
      vec2 off1 = vec2(1.3333333333333333) * direction;
      color += texture2D(image, uv) * 0.29411764705882354;
      color += texture2D(image, uv + (off1 / resolution)) * 0.35294117647058826;
      color += texture2D(image, uv - (off1 / resolution)) * 0.35294117647058826;
      return color;
    }
    vec4 blur9(sampler2D image, vec2 uv, vec2 resolution, vec2 direction) {
      vec4 color = vec4(0.0);
      vec2 off1 = vec2(1.3846153846) * direction;
      vec2 off2 = vec2(3.2307692308) * direction;
      color += texture2D(image, uv) * 0.2270270270;
      color += texture2D(image, uv + (off1 / resolution)) * 0.3162162162;
      color += texture2D(image, uv - (off1 / resolution)) * 0.3162162162;
      color += texture2D(image, uv + (off2 / resolution)) * 0.0702702703;
      color += texture2D(image, uv - (off2 / resolution)) * 0.0702702703;
      return color;
    }
    vec4 blur13(sampler2D image, vec2 uv, vec2 resolution, vec2 direction) {
      vec4 color = vec4(0.0);
      vec2 off1 = vec2(1.411764705882353) * direction;
      vec2 off2 = vec2(3.2941176470588234) * direction;
      vec2 off3 = vec2(5.176470588235294) * direction;
      color += texture2D(image, uv) * 0.1964825501511404;
      color += texture2D(image, uv + (off1 / resolution)) * 0.2969069646728344;
      color += texture2D(image, uv - (off1 / resolution)) * 0.2969069646728344;
      color += texture2D(image, uv + (off2 / resolution)) * 0.09447039785044732;
      color += texture2D(image, uv - (off2 / resolution)) * 0.09447039785044732;
      color += texture2D(image, uv + (off3 / resolution)) * 0.010381362401148057;
      color += texture2D(image, uv - (off3 / resolution)) * 0.010381362401148057;
      return color;
    }

    void main() {
        gl_FragColor =  blur13(tex, texcoord, tex_size, dir);
    }"#;
}
