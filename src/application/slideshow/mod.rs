mod animated_properties;
mod loading;
mod slide;
mod transition;

use std::time::{Duration, Instant};

use anyhow::Result;
use glissade::Easing;
use transition::EaseInOutTransition;
use vek::Vec2;

use self::{
    loading::LoadingSlide,
    slide::{AnimatedSlide, AnimatedSlideProperties, Slide, SlideProperties},
    transition::{DissolveTransition, Transition},
};
use crate::{
    configuration::{InitSlideOptions, Settings},
    graphics::{Drawable, Graphics},
    worker::PreloadedSlide,
};

pub enum Slideshow {
    None,
    Loading(LoadingSlide),
    Single(AnimatedSlide),
    Transitioning(TransitioningSlide),
}

pub struct TransitioningSlide {
    prev: AnimatedSlide,
    next: AnimatedSlide,
}

impl Slideshow {
    pub fn create(graphics: &mut Graphics, config: &Settings) -> Result<Self> {
        match &config.init_slide {
            InitSlideOptions::Empty => Ok(Slideshow::None),
            InitSlideOptions::LoadingCircle(loading_circle_options) => {
                let loading_slide = LoadingSlide::create(graphics, loading_circle_options)?;
                Ok(Slideshow::Loading(loading_slide))
            }
        }
    }

    pub fn should_load_next(&self, time: Instant) -> bool {
        match self {
            Slideshow::None => true,
            Slideshow::Loading(_) => true,
            Slideshow::Single(slide) => slide.is_finished(time),
            Slideshow::Transitioning(_) => false,
        }
    }

    pub fn load_next(
        &mut self,
        graphics: &mut Graphics,
        slide: PreloadedSlide,
        config: &Settings,
        time: Instant,
    ) -> Result<()> {
        let slide = Slide::create(slide, graphics, config)?;
        let mut old_self = Self::None;
        std::mem::swap(self, &mut old_self);
        match old_self {
            Slideshow::None | Slideshow::Loading(_) => {
                *self = Self::to_single(
                    graphics,
                    slide,
                    SlideProperties {
                        zoom: 0.9,
                        ..SlideProperties::default()
                    },
                    config,
                    time,
                )
            }
            Slideshow::Single(mut old)
            | Slideshow::Transitioning(TransitioningSlide {
                prev: _,
                next: mut old,
            }) => {
                let transition = get_random_transition();
                let transition_duration = config.transition_duration;
                transition.ease_out(time, transition_duration, &mut old.animation);
                let mut animation = transition.ease_in(time, transition_duration);
                animation.set_zoom_no_ease(0.9);
                animation.set_text_position_no_ease([0., graphics.get_dimensions().h as f32]);
                let new = AnimatedSlide {
                    slide,
                    animation,
                    finish_at: time,
                };

                *self = Slideshow::Transitioning(TransitioningSlide {
                    prev: old,
                    next: new,
                })
            }
        }
        Ok(())
    }

    // TODO: Test me !
    // Returns the time during wich the application can safely sleep if there is no need to redraw
    pub fn update_get_sleep(
        &mut self,
        graphics: &Graphics,
        config: &Settings,
        time: Instant,
    ) -> Option<Duration> {
        let mut old_self = Self::None;
        let mut max_sleep = None;
        std::mem::swap(self, &mut old_self);
        *self = match old_self {
            Slideshow::None => {
                max_sleep = Some(Duration::MAX);
                old_self
            }
            Slideshow::Loading(ref mut loading) => {
                loading.update(graphics, time);
                old_self
            }
            Slideshow::Single(ref mut slide) => {
                slide.update(time);
                if slide.animation.is_finished(time) {
                    max_sleep = Some(if slide.finish_at >= time {
                        slide.finish_at - time
                    } else {
                        Duration::MAX
                    });
                }
                old_self
            }
            Slideshow::Transitioning(mut t) => {
                if t.is_finished(time) {
                    Self::to_single(
                        graphics,
                        t.next.slide,
                        t.next.animation.get_target(),
                        config,
                        time,
                    )
                } else {
                    t.update(time);
                    Slideshow::Transitioning(t)
                }
            }
        };
        max_sleep
    }

    fn to_single(
        graphics: &Graphics,
        slide: Slide,
        current_properties: SlideProperties,
        config: &Settings,
        start: Instant,
    ) -> Self {
        let mut animation = AnimatedSlideProperties::from(current_properties);
        let display_animation_duration = config
            .max_display_animation_duration
            .unwrap_or(config.display_duration)
            .min(config.display_duration);
        animation.ease_zoom(1.0, start, display_animation_duration, Easing::CubicInOut);
        if let Some(text) = slide.get_text() {
            let size = text.size().as_::<f32>();
            let screen = graphics.get_dimensions().as_::<f32>();

            let target_pos = Vec2::new(screen.w * 0.5 - size.w * 0.5, screen.h - size.h);
            let from_pos = target_pos + Vec2::new(0., size.h);
            animation.set_text_position_no_ease(from_pos.into_array());
            animation.ease_text_position(
                target_pos.into_array(),
                start,
                Duration::from_millis(250),
                Easing::Linear,
            );
        }

        Self::Single(AnimatedSlide {
            slide,
            animation,
            finish_at: start + config.display_duration,
        })
    }
}

impl TransitioningSlide {
    fn is_finished(&self, instant: Instant) -> bool {
        self.prev.animation.is_finished(instant) && self.next.animation.is_finished(instant)
    }

    fn update(&mut self, instant: Instant) {
        self.prev.update(instant);
        self.next.update(instant);
    }
}

impl Drawable for TransitioningSlide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        self.prev.draw(graphics)?;
        self.next.draw(graphics)?;
        Ok(())
    }
}

impl Drawable for Slideshow {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        match self {
            Slideshow::None => Ok(()),
            Slideshow::Loading(slide) => slide.draw(graphics),
            Slideshow::Single(slide) => slide.draw(graphics),
            Slideshow::Transitioning(transitioning_slide) => transitioning_slide.draw(graphics),
        }
    }
}

fn get_random_transition() -> Box<dyn Transition> {
    match rand::random::<u8>() % 2 {
        0 => Box::new(DissolveTransition),
        1 => Box::new(EaseInOutTransition),
        _ => unreachable!(),
    }
}
