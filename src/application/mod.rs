mod fps;
mod config_provider;
mod slideshow;

use std::{rc::Rc, sync::mpsc::TryRecvError, time::Instant};

use anyhow::{Context, Result};
use tokio::sync::watch;
use vek::Extent2;

use self::{fps::FPSCounter, slideshow::Slideshow};
use crate::{
    configuration::AppConfiguration,
    gl::{FutureGlThreadContext, GlContext},
    graphics::{Drawable, Graphics},
    support::ApplicationContext,
    worker::Worker,
};

pub struct Application {
    slides: Slideshow,
    worker: Worker,
    gl: Rc<GlContext>,
    graphics: Graphics,
    config_watch: watch::Receiver<AppConfiguration>,
    config: AppConfiguration,
    fps: Option<FPSCounter>,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "test";

    fn new(
        config_sender: watch::Sender<AppConfiguration>,
        gl: Rc<GlContext>,
        bg_gl: FutureGlThreadContext,
    ) -> Result<Self> {
        let mut config_watch = config_sender.subscribe();
        let config = config_watch.borrow_and_update().clone();

        let mut graphics = Graphics::new(Rc::clone(&gl), config.slideshow.rotation)
            .context("Cannot create Graphics")?;
        let worker = Worker::new(
            config_sender.subscribe(),
            Self::get_ideal_image_size(&gl, &graphics),
            bg_gl,
        );
        let fps = if config.debug.show_fps {
            Some(FPSCounter::new(&mut graphics)?)
        } else {
            None
        };
        let slides = Slideshow::create(&mut graphics, &config)?;
        Ok(Self {
            graphics,
            gl,
            slides,
            worker,
            config_watch,
            config,
            fps,
        })
    }

    fn draw_frame(&mut self) -> Result<()> {
        if let Ok(true) = self.config_watch.has_changed() {
            self.config = self.config_watch.borrow_and_update().clone();
        }
        self.gl.clear();
        let time = Instant::now();
        self.graphics.begin_frame();
        self.worker
            .set_ideal_max_size(Self::get_ideal_image_size(&self.gl, &self.graphics));

        if self.slides.should_load_next(time) {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(error) => Err(error).context("Cannot get next image")?,
                Ok(preloaded_slide) => {
                    self.slides
                        .load_next(&mut self.graphics, preloaded_slide, &self.config, time)
                        .context("Cannot load next frame")?;
                }
            }
        }

        self.slides.update(&self.graphics, &self.config, time);

        if let Some(fps) = &mut self.fps {
            fps.count_frame(time);
        }

        self.graphics.update();

        self.slides.draw(&self.graphics)?;
        if let Some(fps) = &self.fps {
            fps.draw(&self.graphics)?;
        }
        self.gl.swap_buffers()?;
        Ok(())
    }
}

impl Application {
    fn get_ideal_image_size(gl: &GlContext, graphics: &Graphics) -> Extent2<u32> {
        let hw_max = gl.capabilities().max_texture_size;
        let hw_max = Extent2::from(hw_max);

        let fb_dims = graphics.get_dimensions();

        Extent2::min(fb_dims, hw_max)
    }
}
