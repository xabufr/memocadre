mod config_provider;
mod fps;
mod interfaces;
mod slideshow;

use std::{
    rc::Rc,
    sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use config_provider::ConfigProvider;
use log::debug;
use struct_patch::Patch;
use tokio::sync::watch;
use vek::Extent2;

use self::{fps::FPSCounter, slideshow::Slideshow};
use crate::{
    configuration::{Settings, SettingsPatch},
    gl::{FutureGlThreadContext, GlContext},
    graphics::{Drawable, Graphics},
    support::{ApplicationContext, DrawResult},
    worker::Worker,
};

pub enum ControlCommand {
    NextSlide,
    DisplayOn,
    DisplayOff,
    ConfigChanged(SettingsPatch),
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
    config_sender: watch::Sender<Settings>,
    settings: Settings,
    fps: Option<FPSCounter>,
    state: ApplicationState,
    state_notifier: watch::Sender<ApplicationState>,
    control: Receiver<ControlCommand>,
    bg_interfaces_thread: Option<thread::JoinHandle<Result<()>>>,
}

impl ApplicationContext for Application {
    const WINDOW_TITLE: &'static str = "test";

    fn new(gl: Rc<GlContext>, bg_gl: FutureGlThreadContext) -> Result<Self> {
        let provider = ConfigProvider::new();
        let app_config = provider.load_config()?;
        let settings = provider.load_settings()?;
        let config_sender = watch::Sender::new(settings.clone());
        let (control_sender, control) = mpsc::channel();
        let state_notifier = watch::Sender::new(ApplicationState::default());

        let bg_interfaces_thread = interfaces::InterfaceManager::new()
            .start(
                &app_config,
                control_sender,
                state_notifier.clone(),
                config_sender.subscribe(),
            )
            .context("Cannot start interface")?;

        let mut graphics =
            Graphics::new(Rc::clone(&gl), settings.rotation).context("Cannot create Graphics")?;
        let worker = Worker::new(
            config_sender.subscribe(),
            Self::get_ideal_image_size(&gl, &graphics),
            bg_gl,
            app_config.sources,
        );
        let fps = if settings.debug.show_fps {
            Some(FPSCounter::new(&mut graphics)?)
        } else {
            None
        };
        let slides = Slideshow::create(&mut graphics, &settings)?;
        Ok(Self {
            graphics,
            gl,
            slides,
            worker,
            config_sender,
            settings,
            fps,
            control,
            state: state_notifier.clone().borrow().clone(),
            state_notifier,
            bg_interfaces_thread: Some(bg_interfaces_thread),
        })
    }

    fn draw_frame(&mut self) -> Result<DrawResult> {
        self.check_bg_thread()?;
        while let Ok(command) = self.control.try_recv() {
            if let Some(res) = self.handle_command(command) {
                return Ok(res);
            }
        }
        if !self.state.display {
            loop {
                match self.control.recv_timeout(Duration::from_secs(1)) {
                    Ok(command) => {
                        if let Some(res) = self.handle_command(command) {
                            return Ok(res);
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => return Ok(DrawResult::Noop),
                    Err(err) => {
                        Err(err).context("Cannot receive command")?;
                    }
                }
            }
        }
        self.draw()?;
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
            ControlCommand::NextSlide => {
                self.state.force_load_next = true;
                self.state_notifier.send_replace(self.state.clone());
            }
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
            ControlCommand::ConfigChanged(patch) => {
                self.settings.apply(patch);
                self.config_sender.send_replace(self.settings.clone());
            }
        }
        None
    }

    fn check_bg_thread(&mut self) -> Result<()> {
        if let Some(bg) = &self.bg_interfaces_thread {
            if bg.is_finished() {
                let bg = self
                    .bg_interfaces_thread
                    .take()
                    .expect("bg thread is finished");
                match bg.join() {
                    Err(err) => anyhow::bail!("Panic in bg thread: {:?}", err),
                    Ok(Err(err)) => Err(err).context("Error in bg thread: {}")?,
                    Ok(Ok(())) => {
                        debug!("bg interfaces thread finished");
                    }
                };
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), anyhow::Error> {
        self.gl.clear();
        let time = Instant::now();
        self.graphics.begin_frame();
        self.worker
            .set_ideal_max_size(Self::get_ideal_image_size(&self.gl, &self.graphics));
        if self.slides.should_load_next(time) || self.state.force_load_next {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(error) => Err(error).context("Cannot get next image")?,
                Ok(preloaded_slide) => {
                    self.slides
                        .load_next(&mut self.graphics, preloaded_slide, &self.settings, time)
                        .context("Cannot load next frame")?;
                    self.state.force_load_next = false;
                }
            }
        }
        self.slides.update(&self.graphics, &self.settings, time);
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
