use std::borrow::Cow;

use glium::{backend::Facade, index::PrimitiveType, CapabilitiesSource, DrawParameters, Surface};
use glyph_brush::{
    ab_glyph::{point, FontRef},
    BrushAction, BrushError, Extra, GlyphBrush, GlyphBrushBuilder, Section,
};
use log::{debug, warn};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    extra: f32,
    color: [f32; 4],
}

implement_vertex!(Vertex, pos, uv, extra, color,);

pub struct TextDisplay {
    brush: GlyphBrush<[Vertex; 4], Extra, FontRef<'static>>,
    texture: glium::Texture2d,
    vertex: glium::VertexBuffer<Vertex>,
    indices: glium::index::IndexBuffer<u32>,
    program: glium::Program,
}

impl TextDisplay {
    pub fn new(facade: &impl Facade) -> Self {
        let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf")).unwrap();
        let brush = GlyphBrushBuilder::using_font(dejavu).build();
        let (glyph_width, glyph_height) = brush.texture_dimensions();
        static VERTEX_SHADER: &str = include_str!("../shader/vert.glsl");
        static FRAGMENT_SHADER: &str = include_str!("../shader/frag.glsl");
        let program =
            glium::Program::from_source(facade, VERTEX_SHADER, FRAGMENT_SHADER, None).unwrap();

        Self {
            brush,
            // TODO Glium cannot create U8 texture on GLES: https://github.com/glium/glium/issues/1730
            texture: glium::Texture2d::empty(facade, glyph_width, glyph_height).unwrap(),
            vertex: glium::VertexBuffer::empty_dynamic(facade, 4).unwrap(),
            indices: glium::index::IndexBuffer::empty(facade, PrimitiveType::TrianglesList, 0)
                .unwrap(),
            program,
        }
    }

    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.brush.queue(section);
    }

    pub fn update(&mut self, facade: &impl Facade) {
        let action = loop {
            let action = self.brush.process_queued(
                |rect, texture| {
                    let texture: Vec<_> =
                        texture.iter().flat_map(|v| [0 as u8, 0, 0, *v]).collect();
                    let image = glium::texture::RawImage2d {
                        data: std::borrow::Cow::Borrowed(&texture),
                        format: glium::texture::ClientFormat::U8U8U8U8,
                        height: rect.height(),
                        width: rect.width(),
                    };
                    fn rect_to_rect(rect: glyph_brush::Rectangle<u32>) -> glium::Rect {
                        glium::Rect {
                            left: rect.min[0],
                            bottom: rect.min[1],
                            width: rect.width(),
                            height: rect.height(),
                        }
                    }
                    self.texture.write(rect_to_rect(rect), image);
                },
                Vertex::from_vertex,
            );
            match action {
                Err(BrushError::TextureTooSmall { suggested }) => {
                    let max = facade.get_context().get_capabilities().max_texture_size as u32;
                    debug!(
                        "Will increase DisplayText texture size: \
                           Suggested size: {:?}, \
                           Current size: {:?}, \
                           Max size: {:?}",
                        suggested,
                        self.brush.texture_dimensions(),
                        max
                    );
                    let new_dims = if suggested.0 > max || suggested.1 > max {
                        if self.brush.texture_dimensions().0 < max
                            || self.brush.texture_dimensions().1 < max
                        {
                            (max, max)
                        } else {
                            panic!("Glyph brush texture too large for hardware");
                        }
                    } else {
                        suggested
                    };
                    warn!(
                        "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
                        old = self.brush.texture_dimensions(),
                        new = new_dims,
                    );
                    self.brush.resize_texture(new_dims.0, new_dims.1);
                    self.texture = glium::Texture2d::empty(facade, new_dims.0, new_dims.1).unwrap();
                }
                Ok(action) => break action,
            }
        };
        match action {
            BrushAction::Draw(v) => {
                // FIXME Do not recreate vertex/index buffer every time
                let vertex_count = v.len() as u32;
                let v = v.into_iter().flatten().collect::<Vec<_>>();
                self.vertex = glium::VertexBuffer::new(facade, &v).unwrap();
                let indices = (0..vertex_count).fold(
                    Vec::with_capacity(vertex_count as usize * 6),
                    |mut indices, i| {
                        indices.extend_from_slice(&[
                            0 + i * 4,
                            1 + i * 4,
                            2 + i * 4,
                            2 + i * 4,
                            1 + i * 4,
                            3 + i * 4,
                        ]);
                        indices
                    },
                );
                self.indices =
                    glium::IndexBuffer::new(facade, PrimitiveType::TrianglesList, &indices)
                        .unwrap();
            }
            BrushAction::ReDraw => {}
        }
    }

    pub fn draw(&self, frame: &mut impl Surface) {
        // let sampler = glium::uniforms::Sampler::new(&self.glyph_tex)
        //     .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp)
        //     .minify_filter(glium::uniforms::MinifySamplerFilter::Linear)
        //     .magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear);
        let (width, height) = frame.get_dimensions();
        let uniforms = uniform! {
            font_sampler: &self.texture, //sampler,
            transform: glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.).to_cols_array_2d(),
        };
        frame
            .draw(
                &self.vertex,
                &self.indices,
                &self.program,
                &uniforms,
                &DrawParameters {
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                },
            )
            .unwrap();
    }
}

impl Vertex {
    // const INITIAL_AMOUNT: usize = 50_000 * 4; // 200_000 vertices (or, 50_000 glyphs)

    fn from_vertex(
        glyph_brush::GlyphVertex {
            mut tex_coords,
            pixel_coords,
            bounds,
            extra,
        }: glyph_brush::GlyphVertex,
    ) -> [Vertex; 4] {
        let gl_bounds = bounds;

        let mut gl_rect = glyph_brush::ab_glyph::Rect {
            min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
            max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
        };

        // handle overlapping bounds, modify uv_rect to preserve texture aspect
        if gl_rect.max.x > gl_bounds.max.x {
            let old_width = gl_rect.width();
            gl_rect.max.x = gl_bounds.max.x;
            tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.min.x < gl_bounds.min.x {
            let old_width = gl_rect.width();
            gl_rect.min.x = gl_bounds.min.x;
            tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
        }

        if gl_rect.max.y > gl_bounds.max.y {
            let old_height = gl_rect.height();
            gl_rect.max.y = gl_bounds.max.y;
            tex_coords.max.y =
                tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
        }

        if gl_rect.min.y < gl_bounds.min.y {
            let old_height = gl_rect.height();
            gl_rect.min.y = gl_bounds.min.y;
            tex_coords.min.y =
                tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
        }

        // NOTE: This makes so that one `glyph` corresponds
        // to four vertices, which then makes one quad.
        // This is used for maximum compatibility, where
        // some hardware don't support instancing.
        // e.g. OpenGL 2.1, OpenGL ES 2.0, etc.
        [
            Vertex {
                pos: [gl_rect.min.x, gl_rect.max.y],
                uv: [tex_coords.min.x, tex_coords.max.y],
                extra: extra.z,
                color: extra.color,
            },
            Vertex {
                pos: [gl_rect.max.x, gl_rect.max.y],
                uv: [tex_coords.max.x, tex_coords.max.y],
                extra: extra.z,
                color: extra.color,
            },
            Vertex {
                pos: [gl_rect.min.x, gl_rect.min.y],
                uv: [tex_coords.min.x, tex_coords.min.y],
                extra: extra.z,
                color: extra.color,
            },
            Vertex {
                pos: [gl_rect.max.x, gl_rect.min.y],
                uv: [tex_coords.max.x, tex_coords.min.y],
                extra: extra.z,
                color: extra.color,
            },
        ]
    }
}
