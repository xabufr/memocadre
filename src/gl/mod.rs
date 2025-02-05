use std::{cell::RefCell, num::NonZeroU32, ops::Deref, rc::Rc};

use anyhow::{bail, Context as _, Result};
use glow::HasContext;
use glutin::{
    context::{NotCurrentContext, PossiblyCurrentContext},
    prelude::{GlDisplay as _, NotCurrentGlContext},
    surface::{GlSurface as _, Surface, WindowSurface},
};
use vao::VaoBindGuard;
use vek::{Extent2, Rect, Vec2};

use self::shader::ProgramGuard;
pub use self::{shader::Program, texture::Texture};

pub mod buffer_object;
pub mod framebuffer;
pub mod shader;
pub mod texture;
pub mod vao;

pub type GlContext = Rc<GlContextInner>;

pub struct GlContextInner {
    gl: glow::Context,
    capacities: Capabilities,
    info: RefCell<GlContextInfo>,
    surface: Option<Surface<WindowSurface>>,
    context: PossiblyCurrentContext,
}

pub struct FutureGlThreadContext {
    display: glutin::display::Display,
    surface: Option<Surface<WindowSurface>>,
    context: NotCurrentContext,
}

impl FutureGlThreadContext {
    pub fn new(
        surface: Option<Surface<WindowSurface>>,
        context: NotCurrentContext,
        display: glutin::display::Display,
    ) -> Self {
        Self {
            display,
            surface,
            context,
        }
    }

    pub fn activate(self) -> Result<GlContext> {
        let context = match &self.surface {
            Some(surface) => {
                let current = self
                    .context
                    .make_current(&surface)
                    .context("Cannot make context current")?;
                surface
                    .set_swap_interval(
                        &current,
                        glutin::surface::SwapInterval::Wait(
                            NonZeroU32::new(1).expect("should never happen"),
                        ),
                    )
                    .context("Cannot configure swap for GL buffers")?;
                current
            }
            None => match self.context {
                NotCurrentContext::Egl(not_current_context) => PossiblyCurrentContext::Egl(
                    not_current_context
                        .make_current_surfaceless()
                        .context("Cannot make context current")?,
                ),
            },
        };

        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|s| self.display.get_proc_address(s))
        };

        GlContextInner::new(self.surface, context, gl)
    }

    pub fn get_context(&self) -> &NotCurrentContext {
        &self.context
    }
}

impl Deref for GlContextInner {
    type Target = glow::Context;

    fn deref(&self) -> &Self::Target {
        &self.gl
    }
}

pub struct GlContextInfo {
    viewport: Rect<i32, i32>,
}

pub struct Capabilities {
    pub max_texture_size: u32,
}

#[derive(Default)]
pub struct DrawParameters {
    pub blend: Option<BlendMode>,
}

#[derive(Copy, Clone)]
pub struct BlendMode {
    pub alpha: BlendEquation,
    pub color: BlendEquation,
}
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum BlendEquation {
    Add(BlendFunction),
    Subtract(BlendFunction),
    ReverseSubtract(BlendFunction),
}
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct BlendFunction {
    pub src: BlendFactor,
    pub dst: BlendFactor,
}
#[derive(Copy, Clone)]
#[allow(dead_code)]
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
    pub fn to_gl(self) -> u32 {
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
    pub fn to_gl(self) -> u32 {
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
    fn new(
        surface: Option<Surface<WindowSurface>>,
        context: PossiblyCurrentContext,
        gl: glow::Context,
    ) -> Result<Rc<Self>> {
        let dimensions = if let Some(surface) = &surface {
            let width = surface.width().context("cannot get surface width")?;
            let height = surface.height().context("cannot get surface height")?;
            Extent2::new(width as i32, height as i32)
        } else {
            Extent2::zero()
        };
        let viewport = Rect::from((Vec2::zero(), dimensions));
        Ok(Rc::new(Self {
            capacities: Capabilities {
                max_texture_size: unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) } as u32,
            },
            info: RefCell::new(GlContextInfo { viewport }),
            gl,
            surface,
            context,
        }))
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
            }
            self.gl
                .draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, offset);
        }
    }

    pub fn clear(&self) {
        unsafe {
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }

    pub fn current_viewport(&self) -> Rect<i32, i32> {
        self.info.borrow().viewport
    }

    pub fn set_viewport(&self, viewport: Rect<i32, i32>) {
        unsafe {
            self.gl
                .viewport(viewport.x, viewport.y, viewport.w, viewport.h)
        };
        self.info.borrow_mut().viewport = viewport;
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capacities
    }

    pub fn swap_buffers(&self) -> Result<()> {
        if let Some(surface) = &self.surface {
            surface
                .swap_buffers(&self.context)
                .context("Cannot swap buffers")
        } else {
            bail!("Cannot swap buffers on offscreen surface")
        }
    }

    pub fn is_background(&self) -> bool {
        self.surface.is_none()
    }

    pub fn wait(&self) {
        unsafe {
            self.gl.finish();
        }
    }
}
