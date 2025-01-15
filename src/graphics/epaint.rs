use epaint::{
    text::FontDefinitions, Color32, FontId, Fonts, ImageData, Mesh, Pos2, Rect, RectShape,
    TessellationOptions, Tessellator, TextShape,
};
use glium::{backend::Facade, CapabilitiesSource, DrawParameters, Surface};

pub struct EpaintDisplay {
    fonts: Fonts,
    pixels_per_point: f32,
    max_texture_size: usize,
    texture: glium::Texture2d,
    tesselator: Tessellator,
    program: glium::Program,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
    pub fn new<F: Facade>(facade: &F) -> Self {
        let pixels_per_point: f32 = 1.; //1.0833333333333333;
        let max_texture_size = facade.get_context().get_capabilities().max_texture_size as usize;
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
        Self {
            fonts,
            pixels_per_point,
            max_texture_size,
            texture: glium::Texture2d::empty(facade, 0, 0).unwrap(),
            tesselator,
            program: program!(facade,
                              100 => {
                                  vertex: shaders::VERTEX,
                                  fragment: shaders::FRAGMENT,
                              },
            )
            .unwrap(),
        }
    }

    pub fn begin_frame(&self) {
        self.fonts
            .begin_pass(self.pixels_per_point, self.max_texture_size);
    }

    pub fn update<F: Facade>(&mut self, facade: &F, surface: &mut impl Surface) {
        // println!("atlas size: {:?}", self.fonts.font_image_size());
        let galley = self.fonts.layout(
            "test".into(),
            FontId::proportional(32.),
            Color32::WHITE,
            100.,
        );
        let galley2 = self.fonts.layout(
            "Hello World!".into(),
            FontId::proportional(28.),
            Color32::YELLOW,
            1000.,
        );
        if let Some(delta) = self.fonts.font_image_delta() {
            self.update_texture(facade, delta);
        }
        let mut out: Mesh = Default::default();
        self.tesselator.tessellate_text(
            &TextShape::new(Pos2::ZERO, galley.clone(), Color32::WHITE),
            &mut out,
        );
        self.tesselator.tessellate_text(
            &TextShape::new(Pos2::new(100., 200. + 28.), galley2, Color32::WHITE),
            &mut out,
        );

        self.tesselator.tessellate_rect(
            &RectShape::filled(
                epaint::Rect::from_min_size(Pos2::ZERO, epaint::Vec2::new(100., 100.)),
                10.,
                Color32::ORANGE,
            ),
            &mut out,
        );

        let vertex = out
            .vertices
            .into_iter()
            .map(Vertex::from)
            .collect::<Vec<_>>();
        let vertex = glium::VertexBuffer::new(facade, &vertex).unwrap();
        let indices = glium::IndexBuffer::new(
            facade,
            glium::index::PrimitiveType::TrianglesList,
            &out.indices,
        )
        .unwrap();

        let (width, height) = surface.get_dimensions();
        let view = glam::Mat4::orthographic_rh_gl(0., width as _, height as _, 0., -1., 1.);
        let width_in_points = width as f32 / self.pixels_per_point;
        let height_in_points = height as f32 / self.pixels_per_point;
        let uniforms = uniform! {
            tex: self.texture.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear).minify_filter(glium::uniforms::MinifySamplerFilter::Linear).wrap_function(glium::uniforms::SamplerWrapFunction::Clamp),
            view: view.to_cols_array_2d(),
            u_screen_size: [width_in_points, height_in_points],
        };
        surface
            .draw(
                &vertex,
                &indices,
                &self.program,
                &uniforms,
                &DrawParameters {
                    blend: glium::Blend {
                        color: glium::BlendingFunction::Addition {
                            source: glium::LinearBlendingFactor::One,
                            destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
                        },
                        alpha: glium::BlendingFunction::Addition {
                            source: glium::LinearBlendingFactor::OneMinusDestinationAlpha,
                            destination: glium::LinearBlendingFactor::One,
                        },
                        constant_value: (0.0, 0.0, 0.0, 0.0),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
    }

    fn update_texture<F: Facade>(&mut self, facade: &F, delta: epaint::ImageDelta) {
        println!("delta_options: {:?}", delta.options);

        match delta.image {
            epaint::ImageData::Color(_) => println!("Color image"),
            epaint::ImageData::Font(_) => println!("Font image"),
        };
        // println!("delta data: {:?}", delta.image);
        if let Some(pos) = delta.pos {
            unimplemented!()
        } else {
            self.tesselator = Tessellator::new(
                self.pixels_per_point,
                TessellationOptions::default(),
                delta.image.size(),
                Vec::new(),
            );
            self.texture = glium::Texture2d::with_mipmaps(
                facade,
                glium::texture::RawImage2d {
                    data: Self::convert_texture(&delta.image).into(),
                    format: glium::texture::ClientFormat::U8U8U8U8,
                    height: delta.image.height() as _,
                    width: delta.image.width() as _,
                },
                glium::texture::MipmapsOption::NoMipmap,
            )
            .unwrap();
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
