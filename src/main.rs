// #[macro_use]
// extern crate glium;

// mod application;
// mod galery;
// mod graphics;
// mod support;
// mod worker;

use graphics::GlContext;
// fn main() {
//     env_logger::init();
//     application::start();
// }
use raw_window_handle::HasWindowHandle;
use winit::window;

#[macro_use]
extern crate glium;

mod application;
mod galery;
mod graphics;
mod support;
mod worker;

fn main() {
    env_logger::init();
    let (gl, gl_surface, gl_context, shader_version, window, event_loop) = {
        use glutin::{
            config::{ConfigTemplateBuilder, GlConfig},
            context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext},
            display::{GetGlDisplay, GlDisplay},
            surface::{GlSurface, SwapInterval},
        };
        use glutin_winit::{DisplayBuilder, GlWindow};
        use raw_window_handle::HasRawWindowHandle;
        use std::num::NonZeroU32;

        let event_loop = winit::event_loop::EventLoopBuilder::new().build().unwrap();
        let window_attributes = winit::window::Window::default_attributes()
            .with_title("test")
            .with_visible(true);

        let template = ConfigTemplateBuilder::new();

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();
        let raw_window_handle = window.window_handle().unwrap();

        let gl_display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(glutin::context::Version {
                major: 2,
                minor: 0,
            })))
            .build(Some(raw_window_handle.into()));

        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap()
        };

        let attrs = window.build_surface_attributes(Default::default()).unwrap();
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

        let gl =
            unsafe { glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s)) };

        gl_surface
            .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            .unwrap();

        (
            gl,
            gl_surface,
            gl_context,
            "#version 410",
            window,
            event_loop,
        )
    };
    unsafe {
        use glow::HasContext;
        use glutin::prelude::GlSurface;
        use winit::event::{Event, WindowEvent};
        let gl = GlContext::new(gl);
        let mut app = application::GlowApplication::new(&gl);
        let _ = event_loop.run(move |event, elwt| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::Resized(size) => {
                        gl.viewport(0, 0, size.width as _, size.height as _);
                    }
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        gl.clear_color(0., 0., 0., 1.);
                        gl.clear(glow::COLOR_BUFFER_BIT);
                        app.draw(&gl);
                        gl_surface.swap_buffers(&gl_context).unwrap();
                    }
                    _ => (),
                }
            } else if let Event::AboutToWait = event {
                window.request_redraw();
            }
        });
    }
}
