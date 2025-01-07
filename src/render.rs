use conf::LinuxBackend;
use image::DynamicImage;
use miniquad::*;
use std::time::{Duration, Instant};

use crate::graphics::ImageDisplay;

struct Stage {
    ctx: Box<dyn RenderingBackend>,

    image: ImageDisplay,
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

    fn get_frames(&self) -> u32 {
        self.frames
    }

    fn new() -> Self {
        FPSCounter {
            last_fps: 0,
            last_instant: Instant::now(),
            frames: 0,
        }
    }
}

impl Stage {
    pub fn new(image: DynamicImage) -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();

        let image = ImageDisplay::new(ctx.as_mut(), &image);
        Stage {
            image,
            ctx,
            counter: FPSCounter::new(),
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let t = date::now();

        self.counter.count_frame();
        self.ctx.begin_default_pass(Default::default());

        self.image.draw(self.ctx.as_mut());
        self.ctx.end_render_pass();

        self.ctx.commit_frame();
    }
}

pub fn start(image: DynamicImage) {
    let mut conf = conf::Conf::default();
    conf.platform.linux_backend = LinuxBackend::X11Only;

    miniquad::start(conf, move || Box::new(Stage::new(image)));
}
