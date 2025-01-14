#![allow(dead_code)]
use drm::control::ModeTypeFlags;
use drm::control::PageFlipFlags;
use gbm::AsRaw;
use gbm::BufferObjectFlags;
use glissade::Animated;
use glium::Display;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::ContextAttributesBuilder;
use glutin::context::Version;
use glutin::display::GetGlDisplay;
use glutin::display::GlDisplay;
use glutin::prelude::*;
use glutin::surface::SurfaceAttributesBuilder;
use glutin::surface::WindowSurface;
use raw_window_handle::GbmWindowHandle;
use raw_window_handle::{GbmDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::ffi::c_void;
use std::num::NonZeroU32;
use std::ptr::NonNull;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

pub trait ApplicationContext {
    fn draw_frame(&mut self, _display: &Display<WindowSurface>) {}
    fn new(display: &Display<WindowSurface>) -> Self;
    fn update(&mut self) {}
    fn handle_window_event(
        &mut self,
        _event: &winit::event::WindowEvent,
        _window: &winit::window::Window,
    ) {
    }
    const WINDOW_TITLE: &'static str;
}

pub struct State<T> {
    pub display: glium::Display<WindowSurface>,
    pub window: winit::window::Window,
    pub context: T,
}

struct App<T> {
    state: Option<State<T>>,
    visible: bool,
    close_promptly: bool,
}

impl<T: ApplicationContext + 'static> ApplicationHandler<()> for App<T> {
    // The resumed/suspended handlers are mostly for Android compatiblity since the context can get lost there at any point.
    // For convenience's sake, the resumed handler is also called on other platforms on program startup.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(event_loop, self.visible));
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
                if let Some(state) = &self.state {
                    state.display.resize(new_size.into());
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.context.update();
                    state.context.draw_frame(&state.display);
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

pub fn test<T>()
where
    T: ApplicationContext + 'static,
{
    use drm::{
        control::{self, connector, Device as ControlDevice},
        Device as DrmDevice,
    };

    use std::fs::File;
    use std::fs::OpenOptions;

    use std::os::unix::io::AsFd;
    use std::os::unix::io::BorrowedFd;

    #[derive(Debug)]
    /// A simple wrapper for a device node.
    struct Card(File);

    /// Implementing [`AsFd`] is a prerequisite to implementing the traits found
    /// in this crate. Here, we are just calling [`File::as_fd()`] on the inner
    /// [`File`].
    impl AsFd for Card {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.0.as_fd()
        }
    }

    /// With [`AsFd`] implemented, we can now implement [`drm::Device`].
    impl DrmDevice for Card {}
    impl ControlDevice for Card {}

    impl Card {
        /// Simple helper method for opening a [`Card`].
        fn open() -> Self {
            let mut options = OpenOptions::new();
            options.read(true);
            options.write(true);

            // The normal location of the primary device node on Linux
            Card(options.open("/dev/dri/card0").unwrap())
        }
    }

    let devices = glutin::api::egl::device::Device::query_devices()
        .expect("Failed to query devices")
        .collect::<Vec<_>>();
    println!("{:?}", devices);
    println!("Hello, world!");
    let drm_device = Card::open();
    let res = drm_device.resource_handles().unwrap();
    let connector = res
        .connectors()
        .iter()
        .flat_map(|h| drm_device.get_connector(*h, true))
        .find(|c| c.state() == connector::State::Connected)
        .unwrap();
    let mode = connector
        .modes()
        .iter()
        .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
        .unwrap();
    let crtc = connector
        .encoders()
        .iter()
        .flat_map(|h| drm_device.get_encoder(*h))
        .flat_map(|e| e.crtc())
        .flat_map(|c| drm_device.get_crtc(c))
        .next()
        .unwrap();
    let (width, height) = mode.size();
    println!("{:?}", "device opened");
    let device = gbm::Device::new(drm_device).unwrap();
    println!("{:?}", "GBM device opened");
    println!("{}", device.backend_name());
    println!("{:?}", device.get_driver().unwrap());
    let display = unsafe {
        let ptr: NonNull<c_void> = NonNull::new(device.as_raw() as *mut c_void).unwrap();
        let display = RawDisplayHandle::Gbm(GbmDisplayHandle::new(ptr));
        glutin::display::Display::new(display, glutin::display::DisplayApiPreference::Egl).unwrap()
    };
    println!("display: {:?}", display);
    let config = unsafe {
        let configs = display
            .find_configs(
                ConfigTemplateBuilder::new()
                    .prefer_hardware_accelerated(Some(true))
                    .build(),
            )
            .unwrap();
        let configs = configs.collect::<Vec<_>>();
        for config in &configs {
            println!("config: {:?}", config);
            println!(
                "config APIs: {:?}",
                config.api().iter_names().collect::<Vec<_>>()
            );
            println!("config hardware: {:?}", config.hardware_accelerated());
            println!("color: {:?}", config.color_buffer_type());
            println!("float pixels: {:?}", config.float_pixels());
            println!("samples: {:?}", config.num_samples());
        }
        configs.into_iter().next().unwrap()
    };
    let (surface, window, gbm_surface) = unsafe {
        let gbm_surface = device
            .create_surface::<()>(
                width as _,
                height as _,
                gbm::Format::Xrgb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .unwrap();
        let window_handle = RawWindowHandle::Gbm(GbmWindowHandle::new(
            NonNull::new(gbm_surface.as_raw() as *mut c_void).unwrap(),
        ));
        let surface = display
            .create_window_surface(
                &config,
                &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                    window_handle,
                    NonZeroU32::new(width as _).unwrap(),
                    NonZeroU32::new(height as _).unwrap(),
                ),
            )
            .unwrap();
        (surface, window_handle, gbm_surface)
    };
    println!("surface: {:?}", surface);
    let not_current_gl_context = unsafe {
        display
            .create_context(
                &config,
                &ContextAttributesBuilder::new()
                    .with_context_api(glutin::context::ContextApi::Gles(None))
                    .build(Some(window)),
            )
            .unwrap()
    };

    let current_context = not_current_gl_context.make_current(&surface).unwrap();
    println!("current context: {:?}", current_context);
    surface.swap_buffers(&current_context).unwrap();
    let mut bo = unsafe { gbm_surface.lock_front_buffer() }.unwrap();
    let bpp = bo.bpp();
    let fb = device.add_framebuffer(&bo, bpp, bpp).unwrap();
    device
        .set_crtc(
            crtc.handle(),
            Some(fb),
            (0, 0),
            &[connector.handle()],
            Some(*mode),
        )
        .unwrap();

    let display = glium::Display::from_context_surface(current_context, surface).unwrap();
    println!("glium: {:?}", display);
    let mut app = T::new(&display);
    loop {
        app.draw_frame(&display);

        display.swap_buffers().unwrap();
        let next_bo = unsafe { gbm_surface.lock_front_buffer() }.unwrap();
        let fb = device.add_framebuffer(&next_bo, bpp, bpp).unwrap();
        device
            .page_flip(crtc.handle(), fb, PageFlipFlags::EVENT, None)
            .unwrap();

        'outer: loop {
            let mut events = device.receive_events().unwrap();
            for event in &mut events {
                match event {
                    control::Event::PageFlip(event) => {
                        if event.crtc == crtc.handle() {
                            break 'outer;
                        }
                    }
                    _ => (),
                }
            }
        }
        drop(bo);
        bo = next_bo;
    }
}
impl<T: ApplicationContext + 'static> State<T> {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop, visible: bool) -> Self {
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
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();

        // Then the configuration which decides which OpenGL version we'll end up using, here we just use the default which is currently 3.3 core
        // When this fails we'll try and create an ES context, this is mainly used on mobile devices or various ARM SBC's
        // If you depend on features available in modern OpenGL Versions you need to request a specific, modern, version. Otherwise things will very likely fail.
        let window_handle = window
            .window_handle()
            .expect("couldn't obtain window handle");
        let context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(Version::new(2, 0).into()))
            .build(Some(window_handle.into()));
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(Version::new(2, 0).into()))
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
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        // Now we can create our surface, use it to make our context current and finally create our display
        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };
        let current_context = not_current_gl_context
            .unwrap()
            .make_current(&surface)
            .unwrap();
        let display = glium::Display::from_context_surface(current_context, surface).unwrap();

        Self::from_display_window(display, window)
    }

    pub fn from_display_window(
        display: glium::Display<WindowSurface>,
        window: winit::window::Window,
    ) -> Self {
        let context = T::new(&display);
        Self {
            display,
            window,
            context,
        }
    }

    /// Start the event_loop and keep rendering frames until the program is closed
    pub fn run_loop() {
        let event_loop = winit::event_loop::EventLoop::builder()
            .build()
            .expect("event loop building");
        let mut app = App::<T> {
            state: None,
            visible: true,
            close_promptly: false,
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }
}
