use glam::{Mat4, Quat, Vec2, Vec3};
use glium::{backend::Facade, Surface, VertexBuffer};

use super::Vertex2dUv;

pub struct ImageDrawer {
    vertex_buffer: VertexBuffer<Vertex2dUv>,
    index_buffer: glium::IndexBuffer<u16>,
    program: glium::Program,
}

pub struct Sprite {
    pub texture: glium::Texture2d,
    pub position: Vec2,
    pub scale: Vec2,
}

impl Sprite {
    pub fn new(texture: glium::Texture2d) -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::new(texture.width() as _, texture.height() as _),
            texture,
        }
    }

    // Scales the sprite to fit the given dimensions while maintaining aspect ratio
    pub fn resize_respecting_ratio(&mut self, target_size: Vec2) {
        let tex_size = self.get_texture_size();
        let ratio = target_size / tex_size;
        let min = ratio.min_element();
        let ratio = if min < 1. { min } else { ratio.max_element() };
        self.scale = tex_size * ratio;
    }

    pub fn get_texture_size(&self) -> Vec2 {
        Vec2::new(self.texture.width() as _, self.texture.height() as _)
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

impl ImageDrawer {
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
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
            )
            .unwrap(),
        }
    }

    pub fn draw_sprite<S>(&self, surface: &mut S, sprite: &Sprite)
    where
        S: Surface,
    {
        self.draw(surface, &sprite.texture, sprite.position, sprite.scale);
    }

    pub fn draw<S>(&self, surface: &mut S, texture: &glium::Texture2d, position: Vec2, scale: Vec2)
    where
        S: Surface,
    {
        let (width, height) = surface.get_dimensions();
        let model = Mat4::from_scale_rotation_translation(
            Vec3::from((scale, 0.)),
            Quat::IDENTITY,
            Vec3::from((position, 0.)),
        );
        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let uniforms = uniform! {
          model: model.to_cols_array_2d(),
          view: view.to_cols_array_2d(),
          tex: texture,
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
