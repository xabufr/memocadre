use glam::Vec2;
use glium::{backend::Facade, CapabilitiesSource, Surface};
use image::{DynamicImage, GenericImageView};
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::support::{ApplicationContext, State};

use crate::graphics::{blur::ImageBlurr, ImageDisplay};

struct Stage {
    image: ImageDisplay,
    blurr: ImageBlurr,
    current_texture: Option<glium::Texture2d>,
    image_display_start: Instant,
    recv: Receiver<DynamicImage>,
    counter: FPSCounter,
    worker: JoinHandle<()>,
}

struct FPSCounter {
    last_fps: u32,
    last_instant: Instant,
    frames: u32,
}

impl FPSCounter {
    fn count_frame(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_instant;
        if elapsed > Duration::from_secs(1) {
            self.last_fps = self.frames;
            self.last_instant = now;
            self.frames = 0;
            println!("FPS: {}", self.last_fps);
        }
        self.frames += 1;
    }

    fn new() -> Self {
        FPSCounter {
            last_fps: 0,
            last_instant: Instant::now(),
            frames: 0,
        }
    }
}

impl ApplicationContext for Stage {
    const WINDOW_TITLE: &'static str = "test";
    fn new(display: &glium::Display<glutin::surface::WindowSurface>) -> Self {
        println!("{:?}", display.get_context().get_opengl_version_string());
        let (send, recv) = sync_channel(1);
        let worker = thread::spawn(move || {
            use crate::galery::{Galery, ImmichGalery};
            let mut immich = ImmichGalery::new(
                "***REMOVED***",
                "***REMOVED***",
            );
            loop {
                let img = immich.get_next_image();
                send.send(img).unwrap();
            }
        });

        Self {
            image: ImageDisplay::new(display),
            blurr: ImageBlurr::new(display),
            current_texture: None,
            image_display_start: Instant::now(),
            recv,
            counter: FPSCounter::new(),
            worker,
        }
    }

    fn draw_frame(&mut self, display: &glium::Display<glutin::surface::WindowSurface>) {
        let mut frame = display.draw();

        if self.current_texture.is_none()
            || self.image_display_start.elapsed() >= Duration::from_secs(3)
        {
            match self.recv.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(image) => {
                    let (width, height) = image.dimensions();
                    let max = display.get_context().get_capabilities().max_texture_size as u32;
                    let image = if std::cmp::max(width, height) > max {
                        image.resize(max, max, image::imageops::FilterType::Lanczos3)
                    } else {
                        image
                    };
                    let dims = image.dimensions();
                    let texture = glium::Texture2d::new(
                        display,
                        glium::texture::RawImage2d::from_raw_rgb(
                            image.into_rgb8().into_raw(),
                            dims,
                        ),
                    )
                    .unwrap();
                    let texture = self.blurr.blur(display, &texture);
                    self.current_texture = Some(texture);
                    self.image_display_start = Instant::now();
                }
            }
        }

        self.counter.count_frame();

        frame.clear_color(0.0, 0.0, 0.0, 0.0);
        self.current_texture
            .as_ref()
            .inspect(|texture| self.image.draw(&mut frame, Vec2::ZERO, texture));
        frame.finish().unwrap();
    }
}

pub fn start() {
    State::<Stage>::run_loop();
}
