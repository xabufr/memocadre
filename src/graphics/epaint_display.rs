use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use epaint::{
    text::{FontDefinitions, LayoutJob},
    Color32, Fonts, ImageData, Mesh, Shape, TessellationOptions, Tessellator, TextShape,
};
use vek::{Extent2, Mat4, Rect, Vec2};

use super::{Drawable, Graphics, SharedTexture2d};
use crate::gl::{
    buffer_object::{BufferObject, BufferUsage, ElementBufferObject},
    shader::{Program, ProgramGuard},
    texture::{Texture, TextureFiltering, TextureFormat, TextureOptions, TextureWrapMode},
    vao::{BufferInfo, VertexArrayObject},
    BlendMode, DrawParameters, GlContext,
};

pub struct EpaintDisplay {
    fonts: Fonts,
    pixels_per_point: f32,
    max_texture_size: usize,
    texture: Rc<RefCell<Texture>>,
    tesselator: Tessellator,
    program: Rc<Program>,
    gl: Rc<GlContext>,
    containers: Vec<Weak<RefCell<TextContainerInner>>>,
    atlas_updated: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [u8; 4],
    uv: [f32; 2],
}

pub struct TextContainer(Rc<RefCell<TextContainerInner>>);

pub struct ShapeContainer {
    pub position: Vec2<f32>,
    pub opacity_factor: f32,

    vao: VertexArrayObject<Vertex>,
    texture: Option<SharedTexture2d>,
}

impl ShapeContainer {
    fn new(vao: VertexArrayObject<Vertex>, texture: Option<SharedTexture2d>) -> Self {
        Self {
            position: [0., 0.].into(),
            vao,
            texture,
            opacity_factor: 1f32,
        }
    }

    pub fn set_position(&mut self, pos: Vec2<f32>) {
        self.position = pos;
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity_factor = opacity;
    }

    #[inline]
    fn texture(&self) -> Option<&SharedTexture2d> {
        self.texture.as_ref()
    }

    #[inline]
    fn position(&self) -> Vec2<f32> {
        self.position
    }

    #[inline]
    fn opacity(&self) -> f32 {
        self.opacity_factor
    }

    #[inline]
    fn vao(&self) -> &VertexArrayObject<Vertex> {
        &self.vao
    }
}

impl TextContainer {
    fn new(inner: Rc<RefCell<TextContainerInner>>) -> Self {
        Self(inner)
    }

    pub fn set_layout(&self, job: LayoutJob) {
        let mut c = self.0.borrow_mut();
        c.next_layout = Some(job);
        c.is_dirty = true;
    }

    #[allow(dead_code)]
    pub fn get_position(&self) -> Vec2<f32> {
        RefCell::borrow(&self.0).position
    }

    pub fn set_position(&self, pos: Vec2<f32>) {
        self.0.borrow_mut().position = pos;
    }

    pub fn get_bounding_rect(&self) -> Rect<f32, f32> {
        let inner = RefCell::borrow(&self.0);
        if let Some(shape) = &inner.shape {
            let rect = shape.visual_bounding_rect();
            Rect::new(
                rect.min.x + inner.position.x,
                rect.min.y + inner.position.y,
                rect.width(),
                rect.height(),
            )
        } else {
            Rect::new(0., 0., 0., 0.)
        }
    }

    pub fn get_dimensions(&self) -> Extent2<f32> {
        if let Some(shape) = &RefCell::borrow(&self.0).shape {
            let rect = shape.visual_bounding_rect();
            Extent2::new(rect.width(), rect.height())
        } else {
            Extent2::zero()
        }
    }

    pub fn set_opacity(&self, opacity: f32) {
        self.0.borrow_mut().opacity_factor = opacity;
    }

    pub fn force_update(&self, epaint: &mut EpaintDisplay) {
        self.0.borrow_mut().update(epaint);
    }

    #[cfg(test)]
    pub(crate) fn galley(&self) -> Option<std::sync::Arc<epaint::Galley>> {
        self.0
            .borrow()
            .shape
            .as_ref()
            .map(|shape| shape.galley.clone())
    }
}

impl Drawable for TextContainer {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        self.0.borrow().draw(graphics)
    }
}

impl Drawable for ShapeContainer {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics.epaint_display().draw_shape(graphics.view(), self)
    }
}

struct TextContainerInner {
    position: Vec2<f32>,
    text_mesh: Mesh,
    // TODO implement a pool for old VAOs
    text_vao: VertexArrayObject<Vertex>,
    next_layout: Option<LayoutJob>,
    shape: Option<TextShape>,
    opacity_factor: f32,
    is_dirty: bool,
}

impl TextContainerInner {
    #[inline]
    fn draw(&self, graphics: &super::Graphics) -> Result<()> {
        graphics.epaint_display().draw_text(graphics.view(), self)
    }

    fn update(&mut self, epaint: &mut EpaintDisplay) {
        if let Some(job) = self.next_layout.take() {
            let galley = epaint.fonts.layout_job(job);
            self.shape = Some(TextShape::new([0., 0.].into(), galley, Color32::WHITE));
        }
        if self.is_dirty || epaint.atlas_updated {
            self.is_dirty = false;
            self.text_mesh.clear();
            if let Some(shape) = &self.shape {
                epaint
                    .tesselator
                    .tessellate_text(shape, &mut self.text_mesh);

                write_mesh_to_vao(&self.text_mesh, &mut self.text_vao);
            }
        }
    }
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
    pub fn new(gl: Rc<GlContext>) -> Result<Self> {
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

        let program = Program::new(Rc::clone(&gl), shaders::VERTEX, shaders::FRAGMENT)
            .context("Cannot compile epaint shader")?;
        let texture = Texture::empty(Rc::clone(&gl), TextureFormat::Rgba, (0, 0).into())
            .context("Cannot create texture")?;

        Ok(Self {
            fonts,
            pixels_per_point,
            max_texture_size,
            texture: Rc::new(texture.into()),
            tesselator,
            program: Rc::new(program),
            gl,
            containers: vec![],
            atlas_updated: false,
        })
    }

    pub fn begin_frame(&mut self) {
        self.atlas_updated = false;
        self.fonts
            .begin_pass(self.pixels_per_point, self.max_texture_size);
    }

    #[allow(dead_code)]
    pub fn create_shape(
        &mut self,
        shape: Shape,
        texture: Option<SharedTexture2d>,
    ) -> Result<ShapeContainer> {
        let mut mesh = Mesh::default();
        self.tesselator.tessellate_shape(shape, &mut mesh);

        let vbo_data = &[];
        let ebo_data = &[];
        // TODO avoid double buffer init
        let mut vao = self
            .new_vao(vbo_data, ebo_data, BufferUsage::Static)
            .context("Cannot create shape VAO")?;
        write_mesh_to_vao(&mesh, &mut vao);
        Ok(ShapeContainer::new(vao, texture))
    }

    pub fn create_text_container(&mut self) -> Result<TextContainer> {
        let vao = self
            .new_vao(&[], &[], BufferUsage::Dynamic)
            .context("Cannot create text VAO")?;

        let container = TextContainerInner {
            position: [0., 0.].into(),
            text_mesh: Mesh::default(),
            text_vao: vao,
            next_layout: None,
            shape: None,
            opacity_factor: 1f32,
            is_dirty: false,
        };
        let container = Rc::new(RefCell::new(container));
        self.containers.push(Rc::downgrade(&container));
        Ok(TextContainer::new(container))
    }

    fn update_container(&mut self, container: &mut TextContainerInner) {
        if let Some(job) = container.next_layout.take() {
            let galley = self.fonts.layout_job(job);
            container.shape = Some(TextShape::new([0., 0.].into(), galley, Color32::WHITE));
        }
        if container.is_dirty || self.atlas_updated {
            container.is_dirty = false;
            container.text_mesh.clear();
            if let Some(shape) = &container.shape {
                self.tesselator
                    .tessellate_text(shape, &mut container.text_mesh);

                write_mesh_to_vao(&container.text_mesh, &mut container.text_vao);
            }
        }
    }

    pub fn update(&mut self) {
        if let Some(delta) = self.fonts.font_image_delta() {
            self.update_texture(delta);
        }
        let mut i = 0;
        while i < self.containers.len() {
            if let Some(container) = self.containers[i].upgrade() {
                self.update_container(&mut container.borrow_mut());
                i += 1;
            } else {
                self.containers.swap_remove(i);
            }
        }
    }

    fn draw_text(&self, view: Mat4<f32>, text_container: &TextContainerInner) -> Result<()> {
        if text_container.shape.is_none() {
            return Ok(());
        }
        let prog = ProgramGuard::bind(&self.program);
        prog.set_uniform("tex", 0)?;
        self.texture.borrow().bind(Some(0));
        prog.set_uniform("view", view)?;
        let model = Mat4::translation_2d(text_container.position);
        prog.set_uniform("model", model)?;
        prog.set_uniform("opacity", text_container.opacity_factor)?;
        let vao_bind = text_container.text_vao.bind_guard();
        self.gl.draw(
            &vao_bind,
            &prog,
            text_container.text_mesh.indices.len() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
            },
        );
        Ok(())
    }

    pub fn draw_shape(&self, view: Mat4<f32>, shape: &ShapeContainer) -> Result<()> {
        let prog = ProgramGuard::bind(&self.program);
        prog.set_uniform("tex", 0)?;
        if let Some(texture) = shape.texture() {
            texture.bind(Some(0));
        } else {
            self.texture.borrow().bind(Some(0));
        }
        prog.set_uniform("view", view)?;
        let model = Mat4::translation_2d(shape.position());
        prog.set_uniform("model", model)?;
        prog.set_uniform("opacity", shape.opacity())?;
        let vao_bind = shape.vao().bind_guard();
        self.gl.draw(
            &vao_bind,
            &prog,
            shape.vao().element_buffer.size() as _,
            0,
            &DrawParameters {
                blend: Some(BlendMode::alpha()),
            },
        );
        Ok(())
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
        self.texture.borrow_mut().set_options(options);

        let data = Self::convert_texture(&delta.image);
        let dimensions = (delta.image.width() as u32, delta.image.height() as _).into();
        if let Some(pos) = delta.pos {
            self.texture.borrow_mut().write_sub(
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
            self.texture
                .borrow_mut()
                .write(TextureFormat::Rgba, dimensions, &data);
            self.atlas_updated = true;
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

    fn new_vao(
        &mut self,
        vbo_data: &[Vertex],
        ebo_data: &[u32],
        buffer_usage: BufferUsage,
    ) -> Result<VertexArrayObject<Vertex>> {
        let stride = std::mem::size_of::<Vertex>() as i32;
        let buffer_infos = vec![
            BufferInfo {
                location: self.program.get_attrib_location("pos")?,
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, pos) as i32,
            },
            BufferInfo {
                location: self.program.get_attrib_location("color")?,
                data_type: glow::UNSIGNED_BYTE,
                vector_size: 4,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, color) as i32,
            },
            BufferInfo {
                location: self.program.get_attrib_location("uv")?,
                data_type: glow::FLOAT,
                vector_size: 2,
                normalized: false,
                stride,
                offset: memoffset::offset_of!(Vertex, uv) as i32,
            },
        ];
        let mut vbo = BufferObject::new_vertex_buffer(Rc::clone(&self.gl), buffer_usage)
            .context("Cannot create VertexBuffer")?;
        let mut ebo = ElementBufferObject::new_index_buffer(Rc::clone(&self.gl), buffer_usage)
            .context("Cannot create ElementBufferArray")?;
        vbo.write(vbo_data);
        ebo.write(ebo_data);
        VertexArrayObject::new(Rc::clone(&self.gl), vbo, ebo, buffer_infos)
            .context("Cannot create VAO")
    }
}

fn write_mesh_to_vao(mesh: &Mesh, vao: &mut VertexArrayObject<Vertex>) {
    let vertex = mesh
        .vertices
        .iter()
        .copied()
        .map(Vertex::from)
        .collect::<Vec<_>>();

    if vao.vertex_buffer.size() >= vertex.len() {
        vao.vertex_buffer
            .write_sub(0, &vertex)
            .expect("Should never happen: vertex buffer has enough space");
    } else {
        vao.vertex_buffer.write(&vertex);
    }
    if vao.element_buffer.size() >= mesh.indices.len() {
        vao.element_buffer
            .write_sub(0, &mesh.indices)
            .expect("Should never happen: element buffer has enough space");
    } else {
        vao.element_buffer.write(&mesh.indices);
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
    uniform float opacity;

    varying lowp vec2 texcoord;
    varying lowp vec4 texcolor;

    void main() {
        gl_Position = view * model * vec4(pos.xy, 0, 1);
        // gl_Position = vec4(2.0 * pos.x / u_screen_size.x - 1.0,
        //                    1.0 - 2.0 * pos.y / u_screen_size.y,
        //                    0.0,
        //                    1.0);
        texcoord = uv;
        vec4 raw_color = color / 255.0;
        texcolor = vec4(raw_color.rgb, raw_color.a * opacity);
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
