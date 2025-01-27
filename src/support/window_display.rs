use anyhow::{Context, Result};
use glutin::{
    context::{self, PossiblyCurrentContext, Version},
    display::{GetGlDisplay, GlDisplay},
    prelude::*,
    surface::{Surface, WindowSurface},
};
use raw_window_handle::HasWindowHandle;
use std::{num::NonZeroU32, sync::Arc};
use vek::Rect;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    configuration::Conf,
    gl::{GlContext, GlContextInner},
};

use super::ApplicationContext;

pub struct State<T> {
    pub gl: GlContext,
    pub window: winit::window::Window,
    pub context: T,
    gl_context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
}

struct App<T> {
    config: Arc<Conf>,
    state: Option<State<T>>,
    visible: bool,
    close_promptly: bool,
}

impl<T: ApplicationContext + 'static> ApplicationHandler<()> for App<T> {
    // The resumed/suspended handlers are mostly for Android compatiblity since the context can get lost there at any point.
    // For convenience's sake, the resumed handler is also called on other platforms on program startup.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(event_loop, self.visible, self.config.clone()));
        if !self.visible && self.close_promptly {
            event_loop.exit();
        }
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::Resized(new_size) => {
                if let Some(state) = &mut self.state {
                    state.gl.set_viewport(Rect::new(
                        0,
                        0,
                        new_size.width as _,
                        new_size.height as _,
                    ));
                    state.context.resized(new_size.width, new_size.height);
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.context.update();
                    state.context.draw_frame().expect("Cannot draw frame");
                    state
                        .surface
                        .swap_buffers(&state.gl_context)
                        .expect("Cannot swap window buffers");
                    if self.close_promptly {
                        event_loop.exit();
                    }
                }
            }
            // Exit the event loop when requested (by closing the window for example) or when
            // pressing the Esc key.
            winit::event::WindowEvent::CloseRequested
            | winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        state: winit::event::ElementState::Pressed,
                        logical_key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            // Every other event
            ev => {
                if let Some(state) = &mut self.state {
                    state.context.handle_window_event(&ev, &state.window);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}
impl<T: ApplicationContext + 'static> State<T> {
    pub fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        visible: bool,
        config: Arc<Conf>,
    ) -> Self {
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(T::WINDOW_TITLE)
            .with_visible(visible);
        let config_template_builder = glutin::config::ConfigTemplateBuilder::new();
        let display_builder =
            glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        // First we create a window
        let (window, gl_config) = display_builder
            .build(event_loop, config_template_builder, |mut configs| {
                // Just use the first configuration since we don't have any special preferences here
                configs.next().expect("No available GL config")
            })
            .expect("Cannot build GL context");
        let window = window.expect("No window built");

        // Then the configuration which decides which OpenGL version we'll end up using, here we just use the default which is currently 3.3 core
        // When this fails we'll try and create an ES context, this is mainly used on mobile devices or various ARM SBC's
        // If you depend on features available in modern OpenGL Versions you need to request a specific, modern, version. Otherwise things will very likely fail.
        let window_handle = window
            .window_handle()
            .expect("couldn't obtain window handle");
        let context_attributes = context::ContextAttributesBuilder::new()
            .with_context_api(context::ContextApi::Gles(Version::new(2, 0).into()))
            .build(Some(window_handle.into()));
        let fallback_context_attributes = context::ContextAttributesBuilder::new()
            .with_context_api(context::ContextApi::Gles(Version::new(2, 0).into()))
            .build(Some(window_handle.into()));

        let not_current_gl_context = Some(unsafe {
            gl_config
                .display()
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        });

        // Determine our framebuffer size based on the window size, or default to 800x600 if it's invisible
        let (width, height): (u32, u32) = if visible {
            window.inner_size().into()
        } else {
            (800, 600)
        };
        let attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.into(),
            NonZeroU32::new(width).expect("Width cannot be 0"),
            NonZeroU32::new(height).expect("Height cannot be 0"),
        );
        // Now we can create our surface, use it to make our context current and finally create our display
        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("Cannot create window surface")
        };
        let current_context = not_current_gl_context
            .expect("GL context not initialized")
            .make_current(&surface)
            .expect("Cannot activate GL context on window surface");

        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|s| gl_config.display().get_proc_address(s))
        };
        let gl = GlContextInner::new(gl, Rect::new(0, 0, width as _, height as _));
        surface
            .set_swap_interval(
                &current_context,
                glutin::surface::SwapInterval::Wait(
                    NonZeroU32::new(1).expect("should never happen"),
                ),
            )
            .expect("Cannot configure swap for GL buffers");

        Self::from_display_window(gl, window, current_context, surface, config)
    }

    pub fn from_display_window(
        gl: GlContext,
        window: winit::window::Window,
        gl_context: PossiblyCurrentContext,
        surface: Surface<WindowSurface>,
        config: Arc<Conf>,
    ) -> Self {
        let context = T::new(config, GlContext::clone(&gl)).expect("Cannot create application");
        Self {
            gl,
            window,
            context,
            gl_context,
            surface,
        }
    }

    /// Start the event_loop and keep rendering frames until the program is closed
    pub fn run_loop(config: Arc<Conf>) -> Result<()> {
        let event_loop = winit::event_loop::EventLoop::builder()
            .build()
            .context("event loop building")?;
        let mut app = App::<T> {
            config,
            state: None,
            visible: true,
            close_promptly: false,
        };
        event_loop.run_app(&mut app).context("Running application")
    }
}
