use drm::{
    control::{self, connector, Device as ControlDevice, ModeTypeFlags, PageFlipFlags},
    Device as DrmDevice,
};
use gbm::{AsRaw, BufferObjectFlags};
use glutin::{
    config::ConfigTemplateBuilder,
    context::ContextAttributesBuilder,
    display::GlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    num::NonZeroU32,
    os::unix::io::{AsFd, BorrowedFd},
    ptr::NonNull,
};

use super::ApplicationContext;

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
pub fn start_gbm<T>()
where
    T: ApplicationContext + 'static,
{
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
        // device.destroy_framebuffer(fb).unwrap();
    }
}
