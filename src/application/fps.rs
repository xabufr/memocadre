use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId,
};
use log::debug;

use crate::graphics::{Drawable, Graphics, TextContainer};

pub struct FPSCounter {
    last_fps: u32,
    last_instant: Instant,
    frames: u32,
    fps_text: TextContainer,
}

impl FPSCounter {
    pub fn count_frame(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.last_instant;
        if elapsed > Duration::from_secs(1) {
            self.last_fps = self.frames;
            self.last_instant = now;
            self.frames = 0;
            debug!("FPS: {}", self.last_fps);
        }
        self.frames += 1;

        self.fps_text.set_layout(LayoutJob::single_section(
            format!("FPS: {} ({} frames)", self.last_fps, self.frames),
            TextFormat {
                background: Color32::RED,
                ..TextFormat::simple(FontId::proportional(28.), Color32::DEBUG_COLOR)
            },
        ));
    }

    pub fn new(graphics: &mut Graphics) -> Result<Self> {
        let fps_text = graphics
            .create_text_container()
            .context("Cannot create FPS text container")?;
        fps_text.set_position((10., 10.).into());
        Ok(FPSCounter {
            last_fps: 0,
            last_instant: Instant::now(),
            frames: 0,
            fps_text,
        })
    }
}

impl Drawable for FPSCounter {
    fn draw(&self, graphics: &Graphics) -> anyhow::Result<()> {
        graphics.draw(&self.fps_text)
    }
}
