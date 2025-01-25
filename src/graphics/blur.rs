use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    framebuffer::FramebufferObject,
    texture::TextureFormat,
    vao::{BufferInfo, VertexArrayObject},
    GlContext, Program, Texture,
};

use super::Vertex2dUv;

pub struct ImageBlurr {
    vertex_array: VertexArrayObject<Vertex2dUv>,
    program: Program,
    gl: GlContext,
}

#[rustfmt::skip]
const VERTICES: [Vertex2dUv; 4] = [
    Vertex2dUv { pos : [ -1., -1. ], uv: [ 0., 0. ] },
    Vertex2dUv { pos : [  1., -1. ], uv: [ 1., 0. ] },
    Vertex2dUv { pos : [  1.,  1. ], uv: [ 1., 1. ] },
    Vertex2dUv { pos : [ -1.,  1. ], uv: [ 0., 1. ] },
];
const INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
impl ImageBlurr {
    pub fn new(gl: GlContext) -> Self {
        let mut vbo =
            BufferObject::new_vertex_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw);
        let mut ebo =
            ElementBufferObject::new_index_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw);

        let program = Program::new(
            GlContext::clone(&gl),
            shader::VERTEX_BLUR,
            shader::FRAGMENT_BLUR,
        );
        let program = program;
        let pos = program.get_attrib_location("pos");
        let uv = program.get_attrib_location("uv");

        let stride = std::mem::size_of::<Vertex2dUv>() as i32;
        let buffer_infos = vec![
            BufferInfo {
                location: pos,
                vector_size: 2,
                data_type: glow::FLOAT,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex2dUv, pos) as i32,
            },
            BufferInfo {
                location: uv,
                vector_size: 2,
                data_type: glow::FLOAT,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex2dUv, uv) as i32,
            },
        ];

        vbo.write(&VERTICES);
        ebo.write(&INDICES);
        let vao = VertexArrayObject::new(GlContext::clone(&gl), vbo, ebo, buffer_infos);

        Self {
            vertex_array: vao,
            program,
            gl,
        }
    }

    pub fn blur(&self, texture: &Texture) -> Texture {
        let textures = [
            Texture::empty(
                GlContext::clone(&self.gl),
                TextureFormat::RGB,
                texture.size(),
            ),
            Texture::empty(
                GlContext::clone(&self.gl),
                TextureFormat::RGB,
                texture.size(),
            ),
        ];
        let fbos = textures
            .into_iter()
            .map(|texture| FramebufferObject::with_texture(GlContext::clone(&self.gl), texture))
            .collect::<Vec<_>>();

        let mut source_texture = texture;

        let radius: f32 = 6.0;
        let passes = 6;

        let program_bind = self.program.bind();
        let _vao_guard = self.vertex_array.bind_guard();

        program_bind.set_uniform("tex_size", texture.size().as_::<f32>());
        program_bind.set_uniform("tex", 0);

        for i in 0..=passes {
            let radius = radius * (passes - i) as f32 / (passes as f32);

            {
                program_bind.set_uniform("dir", (radius, 0.));
                let _guard = fbos[0].bind_guard();
                source_texture.bind(Some(0));
                self.gl.draw(
                    &_vao_guard,
                    &program_bind,
                    INDICES.len() as _,
                    0,
                    &Default::default(),
                );
            }

            source_texture = fbos[0].get_texture();

            {
                program_bind.set_uniform("dir", (0., radius));
                let _guard = fbos[1].bind_guard();
                source_texture.bind(Some(0));
                self.gl.draw(
                    &_vao_guard,
                    &program_bind,
                    INDICES.len() as _,
                    0,
                    &Default::default(),
                );
            }

            source_texture = fbos[1].get_texture();
        }
        return fbos.into_iter().last().unwrap().into_texture();
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
