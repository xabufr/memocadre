use glium::{backend::Facade, CapabilitiesSource, Surface};
use glam::Vec2;
use image::{DynamicImage, GenericImageView};
use std::cell::Cell;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::support::{ApplicationContext, State};

use crate::graphics::ImageDisplay;

struct Stage {
    image: ImageDisplay,
    current_texture: Option<glium::Texture2d>,
    image_display_start: Instant,
    recv: Receiver<DynamicImage>,
    counter: FPSCounter,
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

thread_local! {
  pub static RECV: Cell<Option<Receiver<DynamicImage>>> = Cell::new(None);
}
impl ApplicationContext for Stage {
    const WINDOW_TITLE: &'static str = "test";
    fn new(display: &glium::Display<glutin::surface::WindowSurface>) -> Self {
        Self {
            image: ImageDisplay::new(display),
            current_texture: None,
            image_display_start: Instant::now(),
            recv: RECV.take().unwrap(),
            counter: FPSCounter::new(),
        }
    }

    fn draw_frame(&mut self, display: &glium::Display<glutin::surface::WindowSurface>) {
        let mut frame = display.draw();
        let n = Instant::now();

        if self.current_texture.is_none()
            || self.image_display_start.elapsed() >= Duration::from_secs(3)
        {
            match self.recv.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(image) => {
                    let (width, height) = image.dimensions();
                    let max = display.get_context().get_capabilities().max_texture_size as u32;
                    let max = 512;
                    let image = if std::cmp::max(width, height) > max {
                        image.resize(max, max, image::imageops::FilterType::Lanczos3)
                    } else {
                        image
                    };
                    let dims = image.dimensions();
                    self.current_texture = glium::Texture2d::new(
                        display,
                        glium::texture::RawImage2d::from_raw_rgb(
                            image.into_rgb8().into_raw(),
                            dims,
                        ),
                    )
                    .unwrap()
                    .into();
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

pub fn start(recv: Receiver<DynamicImage>) {
    RECV.set(Some(recv));
    State::<Stage>::run_loop();
}
