use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId,
};
use glissade::Keyframes;
use log::debug;
use replace_with::replace_with_or_abort;
use std::{
    sync::{mpsc::TryRecvError, Arc},
    time::{Duration, Instant},
};
use vek::{Extent2, Rect, Vec2};

use crate::{
    configuration::Conf,
    galery::ImageWithDetails,
    gl::{GlContext, Texture},
    graphics::{EpaintDisplay, GlowImageBlurr, GlowImageDrawer, SharedTexture2d, Sprite},
    support::{self, ApplicationContext, State},
    worker::Worker,
};

pub struct GlowApplication {
    image_drawer: GlowImageDrawer,
    image_blurr: GlowImageBlurr,
    slides: Slides,
    counter: FPSCounter,
    epaint: EpaintDisplay,
    worker: Worker,
    gl: GlContext,
    config: Arc<Conf>,
}

struct Slide {
    sprites: Vec<Sprite>,
    text: Option<String>,
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
            Slides::Transitioning(t) => t.animation.is_finished(Instant::now()),
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

    pub fn draw(&mut self, gl: &GlContext, image_drawer: &GlowImageDrawer) {
        match self {
            Slides::None => (),
            Slides::Single { slide, start: _ } => slide.draw(gl, image_drawer),
            Slides::Transitioning(transitioning_slide) => {
                let alpha = transitioning_slide.animation.get(Instant::now());
                for s in transitioning_slide.old.sprites.iter_mut() {
                    s.opacity = alpha;
                }
                let alpha = 1. - alpha;
                for s in transitioning_slide.new.sprites.iter_mut() {
                    s.opacity = alpha;
                }
                transitioning_slide.old.draw(gl, image_drawer);
                transitioning_slide.new.draw(gl, image_drawer);
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

impl ApplicationContext for GlowApplication {
    fn new(config: Conf, gl: GlContext) -> Self {
        let config = Arc::new(config);
        let worker = Worker::new(Arc::clone(&config), Self::get_ideal_image_size(&gl));
        worker.start();
        Self {
            image_drawer: GlowImageDrawer::new(&gl),
            image_blurr: GlowImageBlurr::new(&gl),
            counter: FPSCounter::new(),
            epaint: EpaintDisplay::new(GlContext::clone(&gl)),
            gl,
            slides: Slides::None,
            worker,
            config,
        }
    }

    fn draw_frame(&mut self) {
        self.gl.clear();
        self.epaint.begin_frame();
        if self
            .slides
            .should_load_next(self.config.slideshow.display_duration)
        {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(image) => {
                    let slide = self.load_next_frame(&self.gl, image);
                    replace_with_or_abort(&mut self.slides, |slides| {
                        slides.load_next(slide, self.config.slideshow.transition_duration)
                    });
                }
            }
        }

        self.counter.count_frame();

        self.slides.draw(&self.gl, &self.image_drawer);
        // if let Some(slide) = &self.current_slide {
        //     if let Some(text) = &slide.text {
        //         self.epaint.add_text(
        //             [10., 10.],
        //             LayoutJob::single_section(
        //                 text.to_string(),
        //                 TextFormat::simple(FontId::proportional(28.), Color32::WHITE),
        //             ),
        //         );
        //     }
        //     if let Some(animation) = &slide.animation {
        //         if animation.is_finished(frame_time) {}
        //     }
        //     slide.draw(&self.gl, &self.image_drawer);
        // }

        self.epaint.add_text(
            Vec2::new(100., 100.),
            LayoutJob::single_section(
                format!(
                    "FPS: {} ({} frames)",
                    self.counter.last_fps, self.counter.frames
                ),
                TextFormat {
                    background: Color32::RED,
                    ..TextFormat::simple(FontId::proportional(28.), Color32::DEBUG_COLOR)
                },
            ),
        );
        self.epaint.update();
        self.epaint.draw_texts();
    }

    const WINDOW_TITLE: &'static str = "test";
}

impl Slide {
    pub fn draw(&self, gl: &GlContext, image_drawer: &GlowImageDrawer) {
        for sprite in self.sprites.iter() {
            image_drawer.draw_sprite(gl, sprite);
        }
    }
}

pub fn start(config: Conf) {
    let vars = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"];
    let has_window_system = vars.into_iter().any(|v| std::env::var_os(v).is_some());
    if has_window_system {
        State::<GlowApplication>::run_loop(config);
    } else {
        support::start_gbm::<GlowApplication>(config);
    }
}

impl GlowApplication {
    fn get_ideal_image_size(gl: &GlContext) -> Extent2<u32> {
        let hw_max = gl.capabilities().max_texture_size;
        let hw_max = Extent2::from(hw_max);
        let vp = gl.current_viewport();

        let fb_dims = vp.extent().as_();

        let ideal_size = Extent2::min(fb_dims, hw_max);
        return ideal_size;
    }

    fn load_next_frame(&self, gl: &GlContext, image_with_details: ImageWithDetails) -> Slide {
        let image = image_with_details.image;
        let texture = Texture::new_from_image(GlContext::clone(gl), &image);
        let vp = gl.current_viewport();

        let texture = SharedTexture2d::new(texture);
        let mut sprite = Sprite::new(texture.clone());
        let display_size = vp.extent().as_();
        let (width, height) = vp.extent().into_tuple();
        sprite.resize_respecting_ratio(display_size);

        let free_space = display_size.as_() - sprite.size;
        sprite.position = (free_space * 0.5).into();

        let mut sprites = vec![];
        if free_space.reduce_partial_max() > 50.0 {
            let texture_blur = SharedTexture2d::new(self.image_blurr.blur(gl, &texture));
            let mut blur_sprites = [
                Sprite::new(SharedTexture2d::clone(&texture_blur)),
                Sprite::new(texture_blur),
            ];

            for blur_sprite in blur_sprites.iter_mut() {
                blur_sprite.size = sprite.size;
            }

            if free_space.w > 50. {
                blur_sprites[1].position.x = display_size.w as f32 - blur_sprites[1].size.w;

                blur_sprites[0].scissor =
                    Some(Rect::new(0, 0, (free_space.w * 0.5) as i32 + 2, height));

                blur_sprites[1].scissor = Some(Rect::new(
                    width - (free_space.w * 0.5) as i32 - 2,
                    0,
                    (free_space.w * 0.5) as i32 + 2,
                    height,
                ));
            } else {
                blur_sprites[1].position.y = display_size.h as f32 - blur_sprites[1].size.h;

                blur_sprites[0].scissor =
                    Some(Rect::new(0, 0, width, (free_space.h * 0.5) as i32 + 2));
                blur_sprites[1].scissor = Some(Rect::new(
                    0,
                    height - (free_space.h * 0.5) as i32 - 2,
                    width,
                    (free_space.h * 0.5) as i32 + 2,
                ));
            }
            sprites.extend(blur_sprites.into_iter());
        }
        sprites.push(sprite);

        let text = [image_with_details.city, image_with_details.date_time]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let text = if text.is_empty() {
            None
        } else {
            Some(text.join("\n"))
        };

        return Slide {
            sprites,
            text,
            animation: None,
        };
    }
}
