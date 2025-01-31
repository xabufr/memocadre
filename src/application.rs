use std::{
    sync::{mpsc::TryRecvError, Arc},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId,
};
use glissade::Keyframes;
use log::debug;
use replace_with::replace_with_or_abort;
use vek::{Extent2, Rect};

use crate::{
    configuration::{Background, Conf},
    gallery::ImageWithDetails,
    gl::{GlContext, Texture},
    graphics::{epaint_display::TextContainer, Graphics, SharedTexture2d, Sprite},
    support::{self, ApplicationContext, State},
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

struct Slide {
    sprites: Vec<Sprite>,
    text: Option<TextContainer>,
}

struct TransitioningSlide {
    old: Slide,
    new: Slide,
    animation: Box<dyn glissade::Animated<f32, Instant>>,
}

struct FPSCounter {
    last_fps: u32,
    last_instant: Instant,
    frames: u32,
}

enum Slides {
    None,
    Single { slide: Slide, start: Instant },
    Transitioning(TransitioningSlide),
}

impl Slides {
    pub fn should_load_next(&self, display_time: Duration) -> bool {
        match self {
            Slides::None => true,
            Slides::Single { slide: _, start } => start.elapsed() >= display_time,
            Slides::Transitioning(_) => false,
        }
    }

    pub fn load_next(self, slide: Slide, transition_duration: Duration) -> Self {
        match self {
            Slides::None => Slides::Single {
                slide,
                start: Instant::now(),
            },
            Slides::Single {
                slide: old,
                start: _,
            }
            | Slides::Transitioning(TransitioningSlide {
                old: _,
                new: old,
                animation: _,
            }) => Slides::Transitioning(TransitioningSlide {
                old,
                new: slide,
                animation: Box::new(
                    glissade::keyframes::from(1. as f32)
                        .ease_to(0., transition_duration, glissade::Easing::QuarticInOut)
                        .run(Instant::now()),
                ),
            }),
        }
    }

    pub fn update(self) -> Self {
        match self {
            Slides::None => self,
            Slides::Single { .. } => self,
            Slides::Transitioning(mut t) => {
                if t.animation.is_finished(Instant::now()) {
                    t.new.set_opacity(1.);
                    Slides::Single {
                        slide: t.new,
                        start: Instant::now(),
                    }
                } else {
                    let alpha = t.animation.get(Instant::now());
                    t.old.set_opacity(alpha);
                    t.new.set_opacity(1. - alpha);
                    Slides::Transitioning(t)
                }
            }
        }
    }

    pub fn draw(&mut self, graphics: &Graphics) -> Result<()> {
        match self {
            Slides::None => Ok(()),
            Slides::Single { slide, start: _ } => slide.draw(graphics),
            Slides::Transitioning(transitioning_slide) => {
                transitioning_slide.old.draw(graphics)?;
                transitioning_slide.new.draw(graphics)?;
                Ok(())
            }
        }
    }
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
                    let slide = self
                        .load_next_frame(image)
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

        self.slides.draw(&self.graphics)?;
        self.graphics.draw(&self.fps_text)?;
        Ok(())
    }
}

impl Slide {
    pub fn draw(&self, graphics: &Graphics) -> Result<()> {
        for sprite in self.sprites.iter() {
            graphics.draw(sprite)?;
        }
        if let Some(text) = &self.text {
            graphics.draw(text)?;
        }
        Ok(())
    }

    pub fn set_opacity(&mut self, alpha: f32) {
        for sprite in self.sprites.iter_mut() {
            sprite.opacity = alpha;
        }
        self.text.as_mut().map(|text| text.set_opacity(alpha));
    }
}

pub fn start(config: Conf) -> Result<()> {
    let vars = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"];
    let has_window_system = vars.into_iter().any(|v| std::env::var_os(v).is_some());
    let config = Arc::new(config);
    if has_window_system {
        State::<Application>::run_loop(config)
    } else {
        support::start_gbm::<Application>(config)
    }
    .context("While running application")
}

impl Application {
    fn get_ideal_image_size(gl: &GlContext, graphics: &Graphics) -> Extent2<u32> {
        let hw_max = gl.capabilities().max_texture_size;
        let hw_max = Extent2::from(hw_max);

        let fb_dims = graphics.get_dimensions();

        let ideal_size = Extent2::min(fb_dims, hw_max);
        return ideal_size;
    }

    fn load_next_frame(&mut self, image_with_details: ImageWithDetails) -> Result<Slide> {
        let image = image_with_details.image;
        let texture = Texture::new_from_image(GlContext::clone(&self.gl), &image)
            .context("Cannot load main texture")?;

        let texture = SharedTexture2d::new(texture);
        let texture_blur = SharedTexture2d::new(
            self.graphics
                .blurr()
                .blur(self.config.slideshow.blur_options, &texture)
                .context("Cannot blur image")?,
        );

        let mut sprite = Sprite::new(SharedTexture2d::clone(&texture));
        let display_size = self.graphics.get_dimensions();
        let (width, height) = display_size.as_::<i32>().into_tuple();
        sprite.resize_respecting_ratio(display_size);

        let free_space = display_size.as_() - sprite.size;
        sprite.position = (free_space * 0.5).into();

        let mut sprites = vec![];
        if let Background::Burr { min_free_space } = self.config.slideshow.background {
            if free_space.reduce_partial_max() > min_free_space as f32 {
                let mut blur_sprites = [
                    Sprite::new(SharedTexture2d::clone(&texture_blur)),
                    Sprite::new(SharedTexture2d::clone(&texture_blur)),
                ];

                for blur_sprite in blur_sprites.iter_mut() {
                    blur_sprite.size = sprite.size;
                }

                if free_space.w > free_space.h {
                    blur_sprites[0].size.w = (free_space.w * 0.5) as f32;
                    blur_sprites[0].set_sub_rect(Rect::new(
                        0,
                        0,
                        (free_space.w * 0.5) as _,
                        height,
                    ));

                    blur_sprites[1].position.x = display_size.w as f32 - free_space.w * 0.5;
                    blur_sprites[1].size.w = (free_space.w * 0.5) as f32;
                    blur_sprites[1].set_sub_rect(Rect::new(
                        texture.size().w as i32 - (free_space.w * 0.5) as i32,
                        0,
                        (free_space.w * 0.5) as _,
                        height,
                    ));
                } else {
                    blur_sprites[0].size.h = (free_space.h * 0.5) as f32;
                    blur_sprites[0].set_sub_rect(Rect::new(
                        0,
                        0,
                        width,
                        (free_space.h * 0.5) as i32,
                    ));

                    blur_sprites[1].position.y = display_size.h as f32 - free_space.h * 0.5;
                    blur_sprites[1].size.h = (free_space.h * 0.5) as f32;
                    blur_sprites[1].set_sub_rect(Rect::new(
                        0,
                        texture.size().h as i32 - (free_space.h * 0.5) as i32,
                        width,
                        (free_space.h * 0.5) as i32,
                    ));
                }
                sprites.extend(blur_sprites.into_iter());
            }
        }
        sprites.push(sprite);

        let date = image_with_details.date.map(|date| {
            date.date_naive().format_localized(&self.config.slideshow.date.format, self.config.slideshow.date.locale)
                .to_string()
        });
        let text = [image_with_details.city, date]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let text = if text.is_empty() {
            None
        } else {
            Some(text.join("\n"))
        };

        let text = text
            .map(|text| -> Result<_> {
                let mut container = self
                    .graphics
                    .create_text_container()
                    .context("Cannot create text container")?;
                container.set_layout(LayoutJob {
                    halign: epaint::emath::Align::Center,
                    ..LayoutJob::single_section(
                        text,
                        TextFormat {
                            background: Color32::BLACK.linear_multiply(0.5),
                            ..TextFormat::simple(FontId::proportional(28.), Color32::WHITE)
                        },
                    )
                });
                self.graphics.force_text_container_update(&mut container);
                let dims = container.get_dimensions();
                container.set_position(
                    (display_size.w as f32 * 0.5, display_size.h as f32 - dims.h).into(),
                );
                Ok(container)
            })
            .transpose()?;

        return Ok(Slide { sprites, text });
    }
}
