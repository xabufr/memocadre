use bytemuck::{Pod, Zeroable};
use glium::draw_parameters;
use glow::HasContext;
use shader::ProgramGuard;
use std::{cell::RefCell, ops::Deref, rc::Rc};
use vao::{VaoBindGuard, VertexArrayObject};

pub use blur::GlowImageBlurr;
// pub use epaint_display::EpaintDisplay;
pub use image_display::{GlowImageDrawer, Sprite};
// pub use text_display::TextDisplay;
pub use shader::Program;
pub use texture::{SharedTexture2d, Texture};

mod blur;
pub mod buffer_object;
mod image_display;
// pub mod pipeline;
mod shader;
mod texture;
pub mod vao;
// mod epaint_display;
// mod text_display;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);

pub type GlContext = Rc<GlContextInner>;

pub struct GlContextInner {
    gl: glow::Context,
    capacities: Capabilities,
    info: RefCell<GlContextInfo>,
}

impl Deref for GlContextInner {
    type Target = glow::Context;

    fn deref(&self) -> &Self::Target {
        &self.gl
    }
}

pub type Viewport = (i32, i32, i32, i32);
pub struct GlContextInfo {
    viewport: Viewport,
}

pub struct Capabilities {
    pub max_texture_size: u32,
}

pub struct DrawParameters {
    pub blend: Option<BlendMode>,
}

#[derive(Copy, Clone)]
pub struct BlendMode {
    pub alpha: BlendEquation,
    pub color: BlendEquation,
}
#[derive(Copy, Clone)]
pub enum BlendEquation {
    Add(BlendFunction),
    Subtract(BlendFunction),
    ReverseSubtract(BlendFunction),
}
#[derive(Copy, Clone)]
pub struct BlendFunction {
    pub src: BlendFactor,
    pub dst: BlendFactor,
}
#[derive(Copy, Clone)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}
impl BlendMode {
    pub fn alpha() -> Self {
        Self {
            alpha: BlendEquation::Add(BlendFunction {
                src: BlendFactor::SrcAlpha,
                dst: BlendFactor::OneMinusSrcAlpha,
            }),
            color: BlendEquation::Add(BlendFunction {
                src: BlendFactor::SrcAlpha,
                dst: BlendFactor::OneMinusSrcAlpha,
            }),
        }
    }
}
impl BlendEquation {
    pub fn to_gl(&self) -> u32 {
        match self {
            BlendEquation::Add(_) => glow::FUNC_ADD,
            BlendEquation::Subtract(_) => glow::FUNC_SUBTRACT,
            BlendEquation::ReverseSubtract(_) => glow::FUNC_REVERSE_SUBTRACT,
        }
    }
    pub fn get_function(&self) -> &BlendFunction {
        match self {
            BlendEquation::Add(f) => f,
            BlendEquation::Subtract(f) => f,
            BlendEquation::ReverseSubtract(f) => f,
        }
    }
}
impl BlendFactor {
    pub fn to_gl(&self) -> u32 {
        match self {
            BlendFactor::Zero => glow::ZERO,
            BlendFactor::One => glow::ONE,
            BlendFactor::SrcColor => glow::SRC_COLOR,
            BlendFactor::OneMinusSrcColor => glow::ONE_MINUS_SRC_COLOR,
            BlendFactor::DstColor => glow::DST_COLOR,
            BlendFactor::ConstantAlpha => glow::CONSTANT_ALPHA,
            BlendFactor::ConstantColor => glow::CONSTANT_COLOR,
            BlendFactor::OneMinusConstantAlpha => glow::ONE_MINUS_CONSTANT_ALPHA,
            BlendFactor::OneMinusConstantColor => glow::ONE_MINUS_CONSTANT_COLOR,
            BlendFactor::OneMinusDstAlpha => glow::ONE_MINUS_DST_ALPHA,
            BlendFactor::OneMinusDstColor => glow::ONE_MINUS_DST_COLOR,
            BlendFactor::OneMinusSrcAlpha => glow::ONE_MINUS_SRC_ALPHA,
            BlendFactor::SrcAlpha => glow::SRC_ALPHA,
            BlendFactor::DstAlpha => glow::DST_ALPHA,
            BlendFactor::SrcAlphaSaturate => glow::SRC_ALPHA_SATURATE,
        }
    }
}

impl GlContextInner {
    pub fn new(gl: glow::Context, viewport: Viewport) -> Rc<Self> {
        Rc::new(Self {
            capacities: Capabilities {
                max_texture_size: unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) } as u32,
            },
            info: RefCell::new(GlContextInfo { viewport }),
            gl,
        })
    }

    pub fn draw<T>(
        &self,
        _vao: &VaoBindGuard<T>,
        _program: &ProgramGuard,
        count: i32,
        offset: i32,
        draw_parameters: &DrawParameters,
    ) {
        unsafe {
            if let Some(blend) = &draw_parameters.blend {
                self.gl.enable(glow::BLEND);
                self.gl
                    .blend_equation_separate(blend.color.to_gl(), blend.alpha.to_gl());
                self.gl.blend_func_separate(
                    blend.color.get_function().src.to_gl(),
                    blend.color.get_function().dst.to_gl(),
                    blend.alpha.get_function().src.to_gl(),
                    blend.alpha.get_function().dst.to_gl(),
                );
            } else {
                self.gl.disable(glow::BLEND);
            }
            self.gl
                .draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, offset);
        }
    }

    pub fn current_viewport(&self) -> Viewport {
        self.info.borrow().viewport
    }

    pub fn set_viewport(&self, viewport: Viewport) {
        unsafe {
            self.gl
                .viewport(viewport.0, viewport.1, viewport.2, viewport.3)
        };
        self.info.borrow_mut().viewport = viewport;
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capacities
    }
}
