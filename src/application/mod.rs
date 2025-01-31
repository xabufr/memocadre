mod slide;

use std::{
    sync::{mpsc::TryRecvError, Arc},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId,
};
use log::debug;
use replace_with::replace_with_or_abort;
use slide::Slides;
use vek::{Extent2, Rect};

use self::slide::Slide;
use crate::{
    configuration::{Background, Conf},
    gallery::ImageWithDetails,
    gl::{GlContext, Texture},
    graphics::{epaint_display::TextContainer, Graphics, SharedTexture2d, Sprite},
    support::ApplicationContext,
    worker::Worker,
};

pub struct Application {
    slides: Slides,
    counter: FPSCounter,
    worker: Worker,
    gl: GlContext,
    graphics: Graphics,
    config: Arc<Conf>,
    fps_text: TextContainer,
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
            debug!("FPS: {}", self.last_fps);
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

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "test";

    fn new(config: Arc<Conf>, gl: GlContext) -> Result<Self> {
        let mut graphics = Graphics::new(GlContext::clone(&gl), config.slideshow.rotation)
            .context("Cannot create Graphics")?;
        let worker = Worker::new(
            Arc::clone(&config),
            Self::get_ideal_image_size(&gl, &graphics),
        );
        let fps_text = graphics
            .create_text_container()
            .context("Cannot create FPS text container")?;
        fps_text.set_position((10., 10.).into());
        Ok(Self {
            counter: FPSCounter::new(),
            graphics,
            gl,
            slides: Slides::None,
            fps_text,
            worker,
            config,
        })
    }

    fn draw_frame(&mut self) -> Result<()> {
        self.gl.clear();
        self.graphics.begin_frame();
        self.worker
            .set_ideal_max_size(Self::get_ideal_image_size(&self.gl, &self.graphics));

        if self
            .slides
            .should_load_next(self.config.slideshow.display_duration)
        {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(error) => Err(error).context("Cannot get next image")?,
                Ok(image) => {
                    let slide = Slide::create(image, &mut self.graphics, &self.config)
                        .context("Cannot laod next frame")?;
                    replace_with_or_abort(&mut self.slides, |slides| {
                        slides.load_next(slide, self.config.slideshow.transition_duration)
                    });
                }
            }
        }

        replace_with_or_abort(&mut self.slides, |slides| slides.update());
        self.counter.count_frame();

        self.fps_text.set_layout(LayoutJob::single_section(
            format!(
                "FPS: {} ({} frames)",
                self.counter.last_fps, self.counter.frames
            ),
            TextFormat {
                background: Color32::RED,
                ..TextFormat::simple(FontId::proportional(28.), Color32::DEBUG_COLOR)
            },
        ));

        self.graphics.update();

        self.graphics.draw(&self.slides)?;
        self.graphics.draw(&self.fps_text)?;
        Ok(())
    }
}

impl Application {
    fn get_ideal_image_size(gl: &GlContext, graphics: &Graphics) -> Extent2<u32> {
        let hw_max = gl.capabilities().max_texture_size;
        let hw_max = Extent2::from(hw_max);

        let fb_dims = graphics.get_dimensions();

        let ideal_size = Extent2::min(fb_dims, hw_max);
        return ideal_size;
    }
}
