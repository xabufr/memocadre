use std::{
    borrow::Borrow,
    cell::RefCell,
    ops::DerefMut,
    rc::{Rc, Weak},
};

use bytemuck::{Pod, Zeroable};
use epaint::{
    text::{FontDefinitions, LayoutJob},
    Color32, Fonts, ImageData, Mesh, TessellationOptions, Tessellator, TextShape,
};
use vek::{FrustumPlanes, Mat4, Rect, Vec2};

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
    gl: GlContext,
    containers: Vec<Weak<RefCell<TextContainerInner>>>,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [u8; 4],
    uv: [f32; 2],
}

pub struct TextContainer(Rc<RefCell<TextContainerInner>>);

impl TextContainer {
    pub fn set_layout(&self, job: LayoutJob) {
        self.0.borrow_mut().next_layout = Some(job);
    }
    pub fn get_position(&self) -> Vec2<f32> {
        RefCell::borrow(&self.0).position
    }
    pub fn set_position(&self, pos: Vec2<f32>) {
        self.0.borrow_mut().position = pos;
    }
}
struct TextContainerInner {
    position: Vec2<f32>,
    text_mesh: Mesh,
    text_vao: VertexArrayObject<Vertex>,
    next_layout: Option<LayoutJob>,
    shape: Option<TextShape>,
}

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

        let program = Program::new(GlContext::clone(&gl), shaders::VERTEX, shaders::FRAGMENT);

        Self {
            fonts,
            pixels_per_point,
            max_texture_size,
            texture: Texture::empty(GlContext::clone(&gl), TextureFormat::RGBA, (0, 0).into()),
            tesselator,
            program,
            gl,
            containers: vec![],
        }
    }

    pub fn begin_frame(&mut self) {
        self.fonts
            .begin_pass(self.pixels_per_point, self.max_texture_size);
    }

    pub fn create_text_container(&mut self) -> TextContainer {
        let stride = std::mem::size_of::<Vertex>() as i32;
        let buffer_infos = vec![
            BufferInfo {
                location: self.program.get_attrib_location("pos"),
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, pos) as i32,
            },
            BufferInfo {
                location: self.program.get_attrib_location("color"),
                data_type: glow::UNSIGNED_BYTE,
                vector_size: 4,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, color) as i32,
            },
            BufferInfo {
                location: self.program.get_attrib_location("uv"),
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, uv) as i32,
            },
        ];
        let mut vbo =
            BufferObject::new_vertex_buffer(GlContext::clone(&self.gl), BufferUsage::DynamicDraw);
        let mut ebo = ElementBufferObject::new_index_buffer(
            GlContext::clone(&self.gl),
            BufferUsage::DynamicDraw,
        );
        vbo.write(&[]);
        ebo.write(&[]);
        let vao = VertexArrayObject::new(GlContext::clone(&self.gl), vbo, ebo, buffer_infos);

        let container = TextContainerInner {
            position: [0., 0.].into(),
            text_mesh: Mesh::default(),
            text_vao: vao,
            next_layout: None,
            shape: None,
        };
        let container = Rc::new(RefCell::new(container));
        self.containers.push(Rc::downgrade(&container));
        return TextContainer(container);
    }

    fn update_container(&mut self, container: &mut TextContainerInner) {
        if let Some(job) = container.next_layout.take() {
            let galley = self.fonts.layout_job(job);
            container.shape = Some(TextShape::new([0., 0.].into(), galley, Color32::WHITE));
        }
        container.text_mesh.clear();
        if let Some(shape) = &container.shape {
            self.tesselator
                .tessellate_text(shape, &mut container.text_mesh);

            let vertex = container
                .text_mesh
                .vertices
                .iter()
                .copied()
                .map(Vertex::from)
                .collect::<Vec<_>>();

            if container.text_vao.vertex_buffer.size() >= vertex.len() {
                container.text_vao.vertex_buffer.write_sub(0, &vertex);
            } else {
                container.text_vao.vertex_buffer.write(&vertex);
            }
            if container.text_vao.element_buffer.size() >= container.text_mesh.indices.len() {
                container
                    .text_vao
                    .element_buffer
                    .write_sub(0, &container.text_mesh.indices);
            } else {
                container
                    .text_vao
                    .element_buffer
                    .write(&container.text_mesh.indices);
            }
        }
    }

    pub fn draw_container(&self, container: &TextContainer) {
        let container = RefCell::borrow(&container.0);
        if container.borrow().shape.is_none() {
            return;
        }
        let vp = self.gl.current_viewport();
        let view = Mat4::orthographic_without_depth_planes(FrustumPlanes {
            left: 0.,
            right: vp.w as _,
            bottom: vp.h as _,
            top: 0.,
            far: -1.,
            near: 1.,
        });
        let prog = self.program.bind();
        prog.set_uniform("tex", 0);
        self.texture.bind(Some(0));
        prog.set_uniform("view", view);
        let model = Mat4::translation_2d(container.position);
        prog.set_uniform("model", model);
        let vao_bind = container.text_vao.bind_guard();
        self.gl.draw(
            &vao_bind,
            &prog,
            container.text_mesh.indices.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
                ..Default::default()
            },
        );
    }

    pub fn update(&mut self) {
        if let Some(delta) = self.fonts.font_image_delta() {
            self.update_texture(delta);
        }
        let containers = self
            .containers
            .iter()
            .filter_map(|p| p.upgrade())
            .collect::<Vec<_>>();
        self.containers = containers
            .into_iter()
            .map(|p| {
                let mut container = RefCell::borrow_mut(&p);
                self.update_container(container.deref_mut());
                Rc::downgrade(&p)
            })
            .collect::<Vec<_>>();
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
            self.texture.write_sub(
                Rect::from((Vec2::<usize>::from(pos).as_::<u32>(), dimensions)),
                &data,
            );
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
    uniform mat4 model;
    uniform vec2 u_screen_size;

    varying lowp vec2 texcoord;
    varying lowp vec4 texcolor;

    void main() {
        gl_Position = view * model * vec4(pos.xy, 0, 1);
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
