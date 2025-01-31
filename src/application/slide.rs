use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, Pos2, RectShape,
};
use glissade::Keyframes;
use vek::Rect;

use crate::{
    configuration::{Background, Conf},
    gallery::ImageWithDetails,
    graphics::{Drawable, Graphics, ShapeContainer, SharedTexture2d, Sprite, TextContainer},
};

pub struct Slide {
    sprites: Vec<Sprite>,
    text: Option<(TextContainer, ShapeContainer)>,
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
    // TODO: Should refactor this looong method (and test it too!)
    pub fn create(
        image_with_details: ImageWithDetails,
        graphics: &mut Graphics,
        config: &Conf,
    ) -> Result<Self> {
        let image = image_with_details.image;
        let texture = graphics
            .texture_from_image(&image)
            .context("Cannot load main texture")?;

        let texture = SharedTexture2d::new(texture);
        let texture_blur = SharedTexture2d::new(
            graphics
                .blurr()
                .blur(config.slideshow.blur_options, &texture)
                .context("Cannot blur image")?,
        );

        let mut sprite = Sprite::new(SharedTexture2d::clone(&texture));
        let display_size = graphics.get_dimensions();
        let (width, height) = display_size.as_::<i32>().into_tuple();
        sprite.resize_respecting_ratio(display_size);

        let free_space = display_size.as_() - sprite.size;
        sprite.position = (free_space * 0.5).into();

        let mut sprites = vec![];
        if let Background::Burr { min_free_space } = config.slideshow.background {
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
            date.date_naive()
                .format_localized(&config.slideshow.date.format, config.slideshow.date.locale)
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

        let bottom_padding = 10f32;
        let bg_padding = 5f32;
        let text = text
            .map(|text| -> Result<_> {
                let container = {
                    let mut container = graphics
                        .create_text_container()
                        .context("Cannot create text container")?;
                    container.set_layout(LayoutJob {
                        halign: epaint::emath::Align::Center,
                        ..LayoutJob::single_section(
                            text,
                            TextFormat {
                                // background: Color32::BLACK.linear_multiply(0.5),
                                ..TextFormat::simple(FontId::proportional(28.), Color32::WHITE)
                            },
                        )
                    });
                    graphics.force_text_container_update(&mut container);
                    let dims = container.get_dimensions();
                    container.set_position(
                        (
                            display_size.w as f32 * 0.5,
                            display_size.h as f32 - dims.h - bottom_padding - bg_padding,
                        )
                            .into(),
                    );
                    container
                };
                let shape = {
                    let dims = container.get_dimensions() + bg_padding * 2.;
                    let rect = RectShape {
                        blur_width: bg_padding,
                        ..RectShape::filled(
                            epaint::Rect::from_min_size(
                                Pos2::ZERO,
                                epaint::Vec2::new(dims.w, dims.h),
                            ),
                            10f32,
                            Color32::BLACK.linear_multiply(0.5),
                        )
                    };
                    let mut shape = graphics.create_shape(rect.into(), None)?;
                    shape.position = container.get_bounding_rect().position() - bg_padding;
                    shape
                };
                Ok((container, shape))
            })
            .transpose()?;

        return Ok(Slide { sprites, text });
    }

    pub fn set_opacity(&mut self, alpha: f32) {
        for sprite in self.sprites.iter_mut() {
            sprite.opacity = alpha;
        }
        self.text.as_mut().map(|(text, bg)| {
            text.set_opacity(alpha);
            bg.opacity_factor = alpha;
        });
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
        if let Some((text, bg)) = &self.text {
            graphics.draw(bg)?;
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
