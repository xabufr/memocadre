use glam::{Mat4, Quat, UVec2, Vec2, Vec3};
use glium::{backend::Facade, Blend, DrawParameters, Surface, VertexBuffer};

use super::{SharedTexture2d, Vertex2dUv};

pub struct ImageDrawer {
    vertex_buffer: VertexBuffer<Vertex2dUv>,
    index_buffer: glium::IndexBuffer<u16>,
    program: glium::Program,
}

// pub struct URect {
//     pub position: UVec2,
//     pub dimensions: UVec2,
// }
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

    pub texture_rect: Option<glium::Rect>,
}

impl Sprite {
    pub fn new(texture: SharedTexture2d) -> Self {
        Self {
            position: Vec2::ZERO,
            size: Vec2::new(texture.width() as _, texture.height() as _),
            texture,
            opacity: 1.,
            texture_rect: None,
        }
    }

    // Scales the sprite to fit the given dimensions while maintaining aspect ratio
    pub fn resize_respecting_ratio(&mut self, target_size: Vec2) {
        let tex_size = self.get_texture_size();
        let ratio = target_size / tex_size;
        let min = ratio.min_element();
        let ratio = if min < 1. { min } else { ratio.max_element() };
        self.size = tex_size * ratio;
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
        let (width, height) = surface.get_dimensions();
        let model = Mat4::from_scale_rotation_translation(
            Vec3::from((sprite.size, 0.)),
            Quat::IDENTITY,
            Vec3::from((sprite.position, 0.)),
        );
        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let uniforms = uniform! {
          model: model.to_cols_array_2d(),
          view: view.to_cols_array_2d(),
          tex: sprite.texture.as_ref(),
          opacity: sprite.opacity,
        };
        surface
            .draw(
                &self.vertex_buffer,
                &self.index_buffer,
                &self.program,
                &uniforms,
                &DrawParameters {
                    blend: Blend::alpha_blending(),
                    scissor: sprite.texture_rect,
                    ..Default::default()
                },
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
    uniform lowp float opacity;

    void main() {
        gl_FragColor = vec4(texture2D(tex, texcoord).xyz, opacity);
    }"#;
}
