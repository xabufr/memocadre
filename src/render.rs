use glium::Surface;
use image::DynamicImage;
use std::cell::Cell;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::support::{ApplicationContext, State};

use crate::graphics::ImageDisplay;

struct Stage {
    image: Option<ImageDisplay>,
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
            image: None,
            image_display_start: Instant::now(),
            recv: RECV.take().unwrap(),
            counter: FPSCounter::new(),
        }
    }

    fn draw_frame(&mut self, display: &glium::Display<glutin::surface::WindowSurface>) {
        let mut frame = display.draw();
        let n = Instant::now();

        if self.image.is_none() || self.image_display_start.elapsed() >= Duration::from_secs(3) {
            match self.recv.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(next) => {
                    self.image = Some(ImageDisplay::new(display, &next));
                    self.image_display_start = n;
                }
            }
        }

        self.counter.count_frame();

        frame.clear_color(0.0, 0.0, 0.0, 0.0);
        self.image.as_ref().inspect(|img| img.draw(&mut frame));
        frame.finish().unwrap();
    }
}

pub fn start(recv: Receiver<DynamicImage>) {
    RECV.set(Some(recv));
    State::<Stage>::run_loop();
}
