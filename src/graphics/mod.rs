use glam::{Mat4, Vec2, Vec3};
use image::DynamicImage;
use miniquad::*;

#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

pub struct ImageDisplay {
    pipeline: Pipeline,
    bindings: Bindings,
    with: u32,
    height: u32,
    pub position: Vec2,
    pub scale: Vec2,
}

#[rustfmt::skip]
const VERTICES: [Vertex; 4] = [
    Vertex { pos : Vec2 { x: -1., y: -1. }, uv: Vec2 { x: 0., y: 0. } },
    Vertex { pos : Vec2 { x:  1., y: -1. }, uv: Vec2 { x: 1., y: 0. } },
    Vertex { pos : Vec2 { x:  1., y:  1. }, uv: Vec2 { x: 1., y: 1. } },
    Vertex { pos : Vec2 { x: -1., y:  1. }, uv: Vec2 { x: 0., y: 1. } },
];

impl ImageDisplay {
    pub fn new(ctx: &mut Context, image: &DynamicImage) -> Self {
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&VERTICES),
        );

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        let texture = ctx.new_texture_from_rgba8(
            image.width() as u16,
            image.height() as u16,
            &image.clone().into_rgba8().as_raw(),
        );

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![texture],
        };

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
                shader::meta(),
            )
            .unwrap();

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams::default(),
        );

        let ratio = image.width() as f32 / image.height() as f32;
        let scale = if image.width() > image.height() {
            Vec2::new(1., -(image.height() as f32 / image.width() as f32))
        } else {
            Vec2::new(image.width() as f32 / image.height() as f32, -1.)
        };

        Self {
            bindings,
            pipeline,
            with: image.width(),
            height: image.height(),
            position: Vec2::ZERO,
            scale,
        }
    }

    pub fn draw(&self, ctx: &mut dyn RenderingBackend) {
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        let model = Mat4::IDENTITY;
        let model = model * Mat4::from_translation(Vec3::from((self.position, 0.)));
        let model = model * Mat4::from_scale(Vec3::from((self.scale, 0.)));
        ctx.apply_uniforms(UniformsSource::table(&shader::Uniforms { model }));
        ctx.draw(0, 6, 1);
    }
}

mod shader {
    use glam::Mat4;
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    uniform mat4 model;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = model * vec4(in_pos, 0, 1);
        texcoord = in_uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, texcoord);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("model", UniformType::Mat4)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub model: Mat4,
    }
}
