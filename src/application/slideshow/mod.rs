mod animation;
mod slide;

use std::time::{Duration, Instant};

use anyhow::Result;
use glissade::Easing;
use slide::AnimatedSlideProperties;
use vek::Vec2;

use self::slide::{AnimatedSlide, Slide, SlideProperties};
use crate::{
    configuration::Conf,
    graphics::{Drawable, Graphics},
    worker::PreloadedSlide,
};

pub enum Slideshow {
    None,
    Single(AnimatedSlide),
    Transitioning(TransitioningSlide),
}

pub struct TransitioningSlide {
    prev: AnimatedSlide,
    next: AnimatedSlide,
}

impl Slideshow {
    pub fn should_load_next(&self, time: Instant) -> bool {
        match self {
            Slideshow::None => true,
            Slideshow::Single(slide) => slide.animation.is_finished(time),
            Slideshow::Transitioning(_) => false,
        }
    }

    pub fn load_next(
        &mut self,
        graphics: &mut Graphics,
        slide: PreloadedSlide,
        config: &Conf,
        time: Instant,
    ) -> Result<()> {
        let slide = Slide::create(slide, graphics, config)?;
        let mut old_self = Self::None;
        std::mem::swap(self, &mut old_self);
        match old_self {
            Slideshow::None => {
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
                let transition_duration = config.slideshow.transition_duration;
                let easing = Easing::QuarticInOut;
                old.animation
                    .ease_global_opacity(0., time, transition_duration, easing.clone());
                let mut animation = AnimatedSlideProperties::from(SlideProperties {
                    global_opacity: 0.,
                    zoom: 0.9,
                    text_position: [0., graphics.get_dimensions().h as f32],
                });
                animation.ease_global_opacity(1.0, time, transition_duration, easing);
                let new = AnimatedSlide { slide, animation };

                *self = Slideshow::Transitioning(TransitioningSlide {
                    prev: old,
                    next: new,
                })
            }
        }
        Ok(())
    }

    // TODO: Test me !
    pub fn update(&mut self, graphics: &Graphics, config: &Conf, time: Instant) {
        let mut old_self = Self::None;
        std::mem::swap(self, &mut old_self);
        match old_self {
            Slideshow::None => (),
            Slideshow::Single(ref mut slide) => {
                slide.update(time);
                *self = old_self
            }
            Slideshow::Transitioning(mut t) => {
                if t.is_finished(time) {
                    *self = Self::to_single(
                        graphics,
                        t.next.slide,
                        t.next.animation.get_target(),
                        config,
                        time,
                    );
                } else {
                    t.update(time);
                    *self = Slideshow::Transitioning(t);
                }
            }
        }
    }

    fn to_single(
        graphics: &Graphics,
        slide: Slide,
        current_properties: SlideProperties,
        config: &Conf,
        start: Instant,
    ) -> Self {
        let mut animation = AnimatedSlideProperties::from(current_properties);
        animation.ease_zoom(
            1.0,
            start,
            config.slideshow.display_duration,
            Easing::CubicInOut,
        );
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

        Self::Single(AnimatedSlide { slide, animation })
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
            Slideshow::Single(slide) => slide.draw(graphics),
            Slideshow::Transitioning(transitioning_slide) => transitioning_slide.draw(graphics),
        }
    }
}
