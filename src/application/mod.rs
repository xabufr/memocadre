mod config_provider;
mod fps;
mod interfaces;
mod slideshow;

use std::{
    rc::Rc,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::Instant,
};

use anyhow::{Context, Result};
use config_provider::ConfigProvider;
use tokio::sync::watch;
use vek::Extent2;

use self::{fps::FPSCounter, slideshow::Slideshow};
use crate::{
    configuration::Settings,
    gl::{FutureGlThreadContext, GlContext},
    graphics::{Drawable, Graphics},
    support::{ApplicationContext, DrawResult},
    worker::Worker,
};

pub enum ControlCommand {
    // NextSlide,
    DisplayOn,
    DisplayOff,
    // PreviousSlide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationState {
    pub display: bool,
    pub force_load_next: bool,
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self {
            display: true,
            force_load_next: false,
        }
    }
}

pub struct Application {
    slides: Slideshow,
    worker: Worker,
    gl: Rc<GlContext>,
    graphics: Graphics,
    config_watch: watch::Receiver<Settings>,
    config: Settings,
    fps: Option<FPSCounter>,
    state: ApplicationState,
    state_notifier: watch::Sender<ApplicationState>,
    control: Receiver<ControlCommand>,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "test";

    fn new(gl: Rc<GlContext>, bg_gl: FutureGlThreadContext) -> Result<Self> {
        let provider = ConfigProvider::new();
        let sources = provider.load_sources()?;
        let settings = provider.load_settings()?;
        let config_sender = watch::Sender::new(settings);
        let (control_sender, control) = mpsc::channel();
        let state_notifier = watch::Sender::new(ApplicationState::default());

        interfaces::InterfaceManager::new()
            .start(
                control_sender,
                state_notifier.clone(),
                config_sender.clone(),
            )
            .unwrap();

        let mut config_watch = config_sender.subscribe();
        let config = config_watch.borrow_and_update().clone();

        let mut graphics =
            Graphics::new(Rc::clone(&gl), config.rotation).context("Cannot create Graphics")?;
        let worker = Worker::new(
            config_sender.subscribe(),
            Self::get_ideal_image_size(&gl, &graphics),
            bg_gl,
            sources.sources,
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
            control,
            state: state_notifier.clone().borrow().clone(),
            state_notifier,
        })
    }

    fn draw_frame(&mut self) -> Result<DrawResult> {
        if let Ok(true) = self.config_watch.has_changed() {
            self.config = self.config_watch.borrow_and_update().clone();
        }
        while let Ok(command) = self.control.try_recv() {
            if let Some(res) = self.handle_command(command) {
                return Ok(res);
            }
        }
        if !self.state.display {
            while let Ok(command) = self.control.recv() {
                if let Some(res) = self.handle_command(command) {
                    return Ok(res);
                }
            }
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
        Ok(DrawResult::FrameDrawn)
    }
}

impl Application {
    fn get_ideal_image_size(gl: &GlContext, graphics: &Graphics) -> Extent2<u32> {
        let hw_max = gl.capabilities().max_texture_size;
        let hw_max = Extent2::from(hw_max);

        let fb_dims = graphics.get_dimensions();

        Extent2::min(fb_dims, hw_max)
    }
    fn handle_command(&mut self, command: ControlCommand) -> Option<DrawResult> {
        match command {
            // ControlCommand::NextSlide => {
            //     self.state.force_load_next = true;
            // }
            ControlCommand::DisplayOn => {
                if !self.state.display {
                    self.state.display = true;
                    self.state_notifier.send_replace(self.state.clone());
                    return Some(DrawResult::TurnDisplayOn);
                }
            }
            ControlCommand::DisplayOff => {
                if self.state.display {
                    self.state.display = false;
                    self.state_notifier.send_replace(self.state.clone());
                    return Some(DrawResult::TurnDisplayOff);
                }
            }
        }
        None
    }
}
