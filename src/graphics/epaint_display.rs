use std::ops::{RangeFull, RangeTo};

use bytemuck::{Pod, Zeroable};
use epaint::{
    text::{FontDefinitions, LayoutJob},
    Color32, FontId, Fonts, ImageData, Mesh, Pos2, Rect, RectShape, TessellationOptions,
    Tessellator, TextShape,
};
use glam::{UVec2, Vec2};
use glium::{backend::Facade, CapabilitiesSource, Surface};
use mint::{IntoMint, Point2};

use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    texture::{TextureFiltering, TextureFormat, TextureOptions, TextureWrapMode},
    vao::{BufferInfo, VertexArrayObject},
    BlendMode, DrawParameters, GlContext, Program, Texture,
};

pub struct EpaintDisplay {
    fonts: Fonts,
    pixels_per_point: f32,
    max_texture_size: usize,
    texture: Texture,
    tesselator: Tessellator,
    program: Program,
    text_mesh: Mesh,
    text_vao: VertexArrayObject<Vertex>,
    gl: GlContext,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [u8; 4],
    uv: [f32; 2],
}
implement_vertex!(Vertex, pos, color, uv);

impl From<epaint::Vertex> for Vertex {
    fn from(value: epaint::Vertex) -> Self {
        Self {
            pos: [value.pos.x, value.pos.y],
            color: value.color.to_array(),
            uv: [value.uv.x, value.uv.y],
        }
    }
}

impl EpaintDisplay {
    pub fn new(gl: GlContext) -> Self {
        let pixels_per_point: f32 = 1.;
        let max_texture_size = gl.capabilities().max_texture_size as usize;
        let fonts = Fonts::new(
            pixels_per_point,
            max_texture_size,
            FontDefinitions::default(),
        );
        let tesselator = Tessellator::new(
            pixels_per_point,
            TessellationOptions::default(),
            fonts.font_image_size(),
            Vec::new(),
        );

        let vbo = BufferObject::new_vertex_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw);
        let ebo =
            ElementBufferObject::new_index_buffer(GlContext::clone(&gl), BufferUsage::StaticDraw);
        vbo.write(&[]);
        ebo.write(&[]);

        let program = Program::new(GlContext::clone(&gl), shaders::VERTEX, shaders::FRAGMENT);
        let stride = std::mem::size_of::<Vertex>() as i32;
        let buffer_infos = vec![
            BufferInfo {
                location: program.get_attrib_location("pos"),
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, pos) as i32,
            },
            BufferInfo {
                location: program.get_attrib_location("color"),
                data_type: glow::UNSIGNED_BYTE,
                vector_size: 4,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, color) as i32,
            },
            BufferInfo {
                location: program.get_attrib_location("uv"),
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, uv) as i32,
            },
        ];
        let vao = VertexArrayObject::new(GlContext::clone(&gl), vbo, ebo, buffer_infos);

        Self {
            fonts,
            pixels_per_point,
            max_texture_size,
            texture: Texture::empty(GlContext::clone(&gl), TextureFormat::RGBA, (0, 0).into()),
            tesselator,
            program,
            text_mesh: Mesh::default(),
            text_vao: vao,
            gl,
        }
    }

    pub fn begin_frame(&mut self) {
        self.fonts
            .begin_pass(self.pixels_per_point, self.max_texture_size);
        self.text_mesh.clear();
    }

    pub fn draw_texts(&self) {
        let (_, _, width, height) = self.gl.current_viewport();
        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let width_in_points = width as f32 / self.pixels_per_point;
        let height_in_points = height as f32 / self.pixels_per_point;
        // let uniforms = uniform! {
        //     tex: &self.texture, //.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear).minify_filter(glium::uniforms::MinifySamplerFilter::Linear).wrap_function(glium::uniforms::SamplerWrapFunction::Clamp),
        //     view: view.to_cols_array_2d(),
        //     u_screen_size: [width_in_points, height_in_points],
        // };
        let prog = self.program.bind();
        prog.set_uniform("tex", 0);
        self.texture.bind(Some(0));
        prog.set_uniform("view", view);
        // prog.set_uniform("u_screen_size", (width_in_points, height_in_points));
        let vao_bind = self.text_vao.bind_guard();
        self.gl.draw(
            &vao_bind,
            &prog,
            self.text_mesh.indices.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
            },
        );

        // surface
        //     .draw(
        //         &self.text_vertex,
        //         self.text_indices
        //             .slice(RangeTo {
        //                 end: self.text_mesh.indices.len(),
        //             })
        //             .unwrap(),
        //         &self.program,
        //         &uniforms,
        //         &DrawParameters {
        //             blend: glium::Blend {
        //                 color: glium::BlendingFunction::Addition {
        //                     source: glium::LinearBlendingFactor::One,
        //                     destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
        //                 },
        //                 alpha: glium::BlendingFunction::Addition {
        //                     source: glium::LinearBlendingFactor::OneMinusDestinationAlpha,
        //                     destination: glium::LinearBlendingFactor::One,
        //                 },
        //                 constant_value: (0.0, 0.0, 0.0, 0.0),
        //             },
        //             ..Default::default()
        //         },
        //     )
        //     .unwrap();
    }

    // TODO Better interface
    pub fn add_text(&mut self, pos: impl Into<Point2<f32>>, job: LayoutJob) {
        let galley = self.fonts.layout_job(job);
        self.tesselator.tessellate_text(
            &TextShape::new(pos.into().into(), galley, Color32::WHITE),
            &mut self.text_mesh,
        );
    }

    pub fn update(&mut self) {
        if let Some(delta) = self.fonts.font_image_delta() {
            self.update_texture(delta);
        }

        let vertex = self
            .text_mesh
            .vertices
            .iter()
            .copied()
            .map(Vertex::from)
            .collect::<Vec<_>>();
        self.text_vao.vertex_buffer.write(&vertex);
        self.text_vao.element_buffer.write(&self.text_mesh.indices);

        // if self.text_vertex.len() >= vertex.len() {
        //     self.text_vertex
        //         .slice(RangeTo { end: vertex.len() })
        //         .unwrap()
        //         .write(&vertex);
        // } else {
        //     self.text_vertex = glium::VertexBuffer::dynamic(facade, &vertex).unwrap();
        // }
        // if self.text_indices.len() >= self.text_mesh.indices.len() {
        //     self.text_indices
        //         .slice(RangeTo {
        //             end: self.text_mesh.indices.len(),
        //         })
        //         .unwrap()
        //         .write(&self.text_mesh.indices);
        // } else {
        //     self.text_indices = glium::IndexBuffer::dynamic(
        //         facade,
        //         glium::index::PrimitiveType::TrianglesList,
        //         &self.text_mesh.indices,
        //     )
        //     .unwrap();
        // }
    }

    fn update_texture(&mut self, delta: epaint::ImageDelta) {
        let options = TextureOptions {
            min: convert_filter_option(delta.options.minification),
            mag: convert_filter_option(delta.options.magnification),
            wrap: match delta.options.wrap_mode {
                epaint::textures::TextureWrapMode::ClampToEdge => TextureWrapMode::ClampToEdge,
                epaint::textures::TextureWrapMode::Repeat => TextureWrapMode::Repeat,
                epaint::textures::TextureWrapMode::MirroredRepeat => {
                    TextureWrapMode::MirroredRepeat
                }
            },
        };
        self.texture.set_options(options);

        let data = Self::convert_texture(&delta.image);
        let dimensions = (delta.image.width() as u32, delta.image.height() as _).into();
        if let Some(pos) = delta.pos {
            self.texture
                .write_sub(UVec2::new(pos[0] as _, pos[1] as _), dimensions, &data);
        } else {
            self.tesselator = Tessellator::new(
                self.pixels_per_point,
                TessellationOptions::default(),
                delta.image.size(),
                Vec::new(),
            );
            self.texture.write(TextureFormat::RGBA, dimensions, &data);
        }
    }

    fn convert_texture(image: &epaint::image::ImageData) -> Vec<u8> {
        match image {
            ImageData::Font(font_image) => font_image
                .srgba_pixels(None)
                .flat_map(|c| c.to_array())
                .collect(),
            _ => unimplemented!(),
        }
    }
}
fn convert_filter_option(filter: epaint::textures::TextureFilter) -> TextureFiltering {
    match filter {
        epaint::textures::TextureFilter::Nearest => TextureFiltering::Nearest,
        epaint::textures::TextureFilter::Linear => TextureFiltering::Linear,
    }
}

mod shaders {
    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec4 color;
    attribute vec2 uv;

    uniform mat4 view;
    uniform vec2 u_screen_size;

    varying lowp vec2 texcoord;
    varying lowp vec4 texcolor;

    void main() {
        gl_Position = view * vec4(pos.xy, 0, 1);
        // gl_Position = vec4(2.0 * pos.x / u_screen_size.x - 1.0,
        //                    1.0 - 2.0 * pos.y / u_screen_size.y,
        //                    0.0,
        //                    1.0);
        texcoord = uv;
        texcolor = color / 255.0;
    }
    "#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;
    varying lowp vec4 texcolor;

    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, texcoord) * texcolor;
    }"#;
}
