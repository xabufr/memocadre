mod fps;
mod slide;

use std::{rc::Rc, sync::{mpsc::TryRecvError, Arc}};

use anyhow::{Context, Result};
use slide::Slideshow;
use vek::Extent2;

use self::{fps::FPSCounter, slide::Slide};
use crate::{
    configuration::Conf,
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
    config: Arc<Conf>,
    fps: Option<FPSCounter>,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "test";

    fn new(config: Arc<Conf>, gl: Rc<GlContext>, bg_gl: FutureGlThreadContext) -> Result<Self> {
        let mut graphics = Graphics::new(Rc::clone(&gl), config.slideshow.rotation)
            .context("Cannot create Graphics")?;
        let worker = Worker::new(
            Arc::clone(&config),
            Self::get_ideal_image_size(&gl, &graphics),
            bg_gl,
        );
        let fps = if config.debug.show_fps {
            Some(FPSCounter::new(&mut graphics)?)
        } else {
            None
        };
        Ok(Self {
            graphics,
            gl,
            slides: Slideshow::None,
            worker,
            config,
            fps,
        })
    }

    fn draw_frame(&mut self) -> Result<()> {
        self.gl.clear();
        self.graphics.begin_frame();
        self.worker
            .set_ideal_max_size(Self::get_ideal_image_size(&self.gl, &self.graphics));

        if self.slides.should_load_next() {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(error) => Err(error).context("Cannot get next image")?,
                Ok(image) => {
                    let slide = Slide::create(image, &mut self.graphics, &self.config)
                        .context("Cannot laod next frame")?;

                    self.slides.load_next(slide, &self.config);
                }
            }
        }

        self.slides.update(&self.config);

        if let Some(fps) = &mut self.fps {
            fps.count_frame();
        }

        self.graphics.update();

        self.slides.draw(&self.graphics)?;
        if let Some(fps) = &self.fps {
            fps.draw(&self.graphics)?;
        }
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
