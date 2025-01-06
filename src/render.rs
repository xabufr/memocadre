use conf::LinuxBackend;
use image::DynamicImage;
use miniquad::*;
use std::time::{Duration, Instant};

#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}
#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,

    pipeline: Pipeline,
    bindings: Bindings,
    counter: FPSCounter,
}

struct FPSCounter {
    last_fps: u32,
    last_instant: Instant,
    frames: u32,
}

impl FPSCounter {
    fn count_frame(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_instant;
        if elapsed > Duration::from_secs(1) {
            self.last_fps = self.frames;
            self.last_instant = now;
            self.frames = 0;
            println!("FPS: {}", self.last_fps);
        }
        self.frames += 1;
    }

    fn get_frames(&self) -> u32 {
        self.frames
    }

    fn new() -> Self {
        FPSCounter {
            last_fps: 0,
            last_instant: Instant::now(),
            frames: 0,
        }
    }
}

impl Stage {
    pub fn new(image: DynamicImage) -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();

        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : Vec2 { x: -0.5, y: -0.5 }, uv: Vec2 { x: 0., y: 1. } },
            Vertex { pos : Vec2 { x:  0.5, y: -0.5 }, uv: Vec2 { x: 1., y: 1. } },
            Vertex { pos : Vec2 { x:  0.5, y:  0.5 }, uv: Vec2 { x: 1., y: 0. } },
            Vertex { pos : Vec2 { x: -0.5, y:  0.5 }, uv: Vec2 { x: 0., y: 0. } },
        ];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        println!("{:?}", ctx.info());
        unsafe {
            use miniquad::graphics::raw_gl::*;
            let mut max_size: i32 = 0;
            glGetIntegerv(GL_MAX_TEXTURE_SIZE, &mut max_size);
            println!("max texture size: {}", max_size);
        }
        // let image = image.resize(2048, 2048, image::imageops::FilterType::Lanczos3);
        let texture = ctx.new_texture_from_rgba8(
            image.width() as u16,
            image.height() as u16,
            &image.into_rgba8().as_raw(),
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

        Stage {
            pipeline,
            bindings,
            ctx,
            counter: FPSCounter::new(),
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let t = date::now();

        self.counter.count_frame();
        self.ctx.begin_default_pass(Default::default());

        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        for i in 0..1 {
            let t = t + i as f64 * 0.3;

            self.ctx
                .apply_uniforms(UniformsSource::table(&shader::Uniforms {
                    offset: (t.sin() as f32 * 0.5, (t * 3.).cos() as f32 * 0.5),
                }));
            self.ctx.draw(0, 6, 1);
        }
        self.ctx.end_render_pass();

        self.ctx.commit_frame();
    }
}

pub fn start(image: DynamicImage) {
    let mut conf = conf::Conf::default();
    conf.platform.linux_backend = LinuxBackend::X11Only;

    miniquad::start(conf, move || Box::new(Stage::new(image)));
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    uniform vec2 offset;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(in_pos + offset, 0, 1);
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
                uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
    }
}
