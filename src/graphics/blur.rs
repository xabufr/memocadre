use std::time::Instant;

use glam::{Mat4, Quat, Vec2, Vec3};
use glium::{
    backend::Facade, BlitMask, BlitTarget, CapabilitiesSource, Rect, Surface, VertexBuffer,
};
use image::{DynamicImage, GenericImageView};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex, pos, uv);

pub struct ImageBlurr {
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    program: glium::Program,
}

#[rustfmt::skip]
const VERTICES: [Vertex; 4] = [
    Vertex { pos : [ -1., -1. ], uv: [ 0., 0. ] },
    Vertex { pos : [  1., -1. ], uv: [ 1., 0. ] },
    Vertex { pos : [  1.,  1. ], uv: [ 1., 1. ] },
    Vertex { pos : [ -1.,  1. ], uv: [ 0., 1. ] },
];
const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl ImageBlurr {
    pub fn new<F>(facade: &F) -> Self
    where
        F: Facade,
    {
        Self {
            vertex_buffer: glium::VertexBuffer::new(facade, &VERTICES).unwrap(),
            index_buffer: glium::IndexBuffer::new(
                facade,
                glium::index::PrimitiveType::TrianglesList,
                &INDICES,
            )
            .unwrap(),
            program: program!(facade,
                100 => {
                    vertex: shader::VERTEX_BLUR,
                    fragment: shader::FRAGMENT_BLUR,
                },
            )
            .unwrap(),
        }
    }

    pub fn blur(&self, facade: &impl Facade, texture: &glium::Texture2d) -> glium::Texture2d {
        use glium::framebuffer::SimpleFrameBuffer;
        let textures = [
            glium::Texture2d::empty(facade, texture.width(), texture.height()).unwrap(),
            glium::Texture2d::empty(facade, texture.width(), texture.height()).unwrap(),
        ];
        let mut fbos = [
            SimpleFrameBuffer::new(facade, &textures[0]).unwrap(),
            SimpleFrameBuffer::new(facade, &textures[1]).unwrap(),
        ];
        let mut source_texture = texture;

        let radius: f32 = 6.0;
        let size = (texture.width() as f32, texture.height() as f32);
        let passes = 6;
        for i in 0..=passes {
            let radius = radius * (passes - i) as f32 / (passes as f32);
            let uniforms = uniform! {
              tex_size: size,
              tex: source_texture,
              dir: (radius, 0.),
            };
            fbos[0]
                .draw(
                    &self.vertex_buffer,
                    &self.index_buffer,
                    &self.program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
            source_texture = &textures[0];

            let uniforms = uniform! {
              tex_size: size,
              tex: source_texture,
              dir: (0., radius),
            };
            fbos[1]
                .draw(
                    &self.vertex_buffer,
                    &self.index_buffer,
                    &self.program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
            source_texture = &textures[1];
        }

        let [_, texture] = textures;
        return texture;
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
