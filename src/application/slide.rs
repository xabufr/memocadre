use std::time::{Duration, Instant};

use anyhow::Result;
use glissade::Keyframes;

use crate::graphics::{epaint_display::TextContainer, Drawable, Graphics, Sprite};

pub struct Slide {
    sprites: Vec<Sprite>,
    text: Option<TextContainer>,
}

pub enum Slides {
    None,
    Single { slide: Slide, start: Instant },
    Transitioning(TransitioningSlide),
}
pub struct TransitioningSlide {
    old: Slide,
    new: Slide,
    animation: Box<dyn glissade::Animated<f32, Instant>>,
}

impl Slide {
    pub fn new(sprites: Vec<Sprite>, text: Option<TextContainer>) -> Self {
        Self { sprites, text }
    }

    pub fn set_opacity(&mut self, alpha: f32) {
        for sprite in self.sprites.iter_mut() {
            sprite.opacity = alpha;
        }
        self.text.as_mut().map(|text| text.set_opacity(alpha));
    }
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
}

impl Drawable for Slide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        for sprite in self.sprites.iter() {
            graphics.draw(sprite)?;
        }
        if let Some(text) = &self.text {
            graphics.draw(text)?;
        }
        Ok(())
    }
}

impl Drawable for Slides {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        match self {
            Slides::None => Ok(()),
            Slides::Single { slide, start: _ } => graphics.draw(slide),
            Slides::Transitioning(transitioning_slide) => {
                graphics.draw(&transitioning_slide.old)?;
                graphics.draw(&transitioning_slide.new)?;
                Ok(())
            }
        }
    }
}
