use glam::{Mat4, Quat, Vec2, Vec3};
use glium::{backend::Facade, CapabilitiesSource, Surface, VertexBuffer};
use image::{DynamicImage, GenericImageView};

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex, pos, uv);

pub struct ImageDisplay {
    pub position: Vec2,
    pub scale: Vec2,
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    program: glium::Program,
    texture: glium::Texture2d,
}

#[rustfmt::skip]
const VERTICES: [Vertex; 4] = [
    Vertex { pos : [ 0., 0. ], uv: [ 0.,  0. ] },
    Vertex { pos : [  1., 0. ], uv: [ 1.,  0. ] },
    Vertex { pos : [  1.,  1. ], uv: [ 1.,  1. ] },
    Vertex { pos : [ 0.,  1. ], uv: [ 0.,  1. ] },
];
const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

impl ImageDisplay {
    pub fn new<F>(facade: &F, image: &DynamicImage) -> Self
    where
        F: Facade,
    {
        // let scale = if image.width() > image.height() {
        //     Vec2::new(1., -(image.height() as f32 / image.width() as f32))
        // } else {
        //     Vec2::new(image.width() as f32 / image.height() as f32, -1.)
        // };

        let (width, height) = image.dimensions();
        let max = facade.get_context().get_capabilities().max_texture_size as u32;
        let max = 512;
        let image = if std::cmp::max(width, height) > max {
            image.resize(max, max, image::imageops::FilterType::Lanczos3)
        } else {
            image.clone()
        };
        let (width, height) = image.dimensions();
        let scale = Vec2::new(image.width() as _, image.height() as _);

        Self {
            position: Vec2::new(100., 100.),
            scale,
            vertex_buffer: glium::VertexBuffer::new(facade, &VERTICES).unwrap(),
            index_buffer: glium::IndexBuffer::new(
                facade,
                glium::index::PrimitiveType::TrianglesList,
                &INDICES,
            )
            .unwrap(),
            program: program!(facade,
                100 => {
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
            )
            .unwrap(),
            texture: glium::Texture2d::new(
                facade,
                glium::texture::RawImage2d::from_raw_rgb(
                    image.into_rgb8().into_raw(),
                    (width, height),
                ),
            )
            .unwrap(),
        }
    }

    pub fn draw<S>(&self, surface: &mut S)
    where
        S: Surface,
    {
        let (width, height) = surface.get_dimensions();
        let model = Mat4::IDENTITY;
        let model = model * Mat4::from_translation(Vec3::from((self.position, 0.)));
        // let model =
        // model * Mat4::from_translation(Vec3::new(width as f32 * 0.5, height as f32 * 0.5, 0.));
        // let model = model * Mat4::from_scale(Vec3::from((self.scale, 0.)));
        let model = Mat4::from_scale_rotation_translation(
            Vec3::from((self.scale, 0.)),
            Quat::IDENTITY,
            Vec3::from((self.position, 0.))

        );
        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let uniforms = uniform! {
          model: model.to_cols_array_2d(),
          view: view.to_cols_array_2d(),
          tex: &self.texture,
        };
        surface
            .draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
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

    void main() {
        gl_FragColor = texture2D(tex, texcoord);
    }"#;
}
