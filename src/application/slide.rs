use std::time::Instant;

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, Pos2, RectShape,
};
use glissade::{Keyframes, Mix};
use smart_default::SmartDefault;
use vek::Rect;

use crate::{
    configuration::{Background, Conf},
    graphics::{Drawable, Graphics, ShapeContainer, SharedTexture2d, Sprite, TextContainer},
    worker::PreloadedSlide,
};

pub struct Slide {
    main_sprite: Sprite,
    background: Option<[Sprite; 2]>,
    text: Option<TextWithBackground>,
}

pub enum Slides {
    None,
    Single(AnimatedSlide),
    Transitioning(TransitioningSlide),
}

pub struct TransitioningSlide {
    old: AnimatedSlide,
    new: AnimatedSlide,
}

pub struct AnimatedSlide {
    slide: Slide,
    animation: Box<dyn glissade::Animated<SlideProperties, Instant>>,
}

struct TextWithBackground {
    container: TextContainer,
    background: ShapeContainer,
}

#[derive(Mix, SmartDefault, Clone)]
struct SlideProperties {
    #[default(1_f32)]
    global_opacity: f32,
    #[default(1_f32)]
    zoom: f32,
}

impl AnimatedSlide {
    fn update(&mut self, instant: Instant) {
        let properties = self.animation.get(instant);
        self.slide.apply(properties);
    }
}

impl Slide {
    // TODO: Should refactor this looong method (and test it too!)
    pub fn create(
        preloaded_slide: PreloadedSlide,
        graphics: &mut Graphics,
        config: &Conf,
    ) -> Result<Self> {
        let texture = graphics.texture_from_detached(preloaded_slide.texture);
        let texture = SharedTexture2d::new(texture);
        let texture_blur =
            SharedTexture2d::new(graphics.texture_from_detached(preloaded_slide.blurred_texture));

        let mut main_sprite = Sprite::new(SharedTexture2d::clone(&texture));
        let display_size = graphics.get_dimensions();
        let (width, height) = display_size.as_::<i32>().into_tuple();
        main_sprite.resize_respecting_ratio(display_size);

        let free_space = display_size.as_() - main_sprite.size;
        main_sprite.position = (free_space * 0.5).into();

        let mut background = None;
        if let Background::Burr { min_free_space } = config.slideshow.background {
            if free_space.reduce_partial_max() > min_free_space as f32 {
                let mut blur_sprites = [
                    Sprite::new(SharedTexture2d::clone(&texture_blur)),
                    Sprite::new(SharedTexture2d::clone(&texture_blur)),
                ];

                for blur_sprite in blur_sprites.iter_mut() {
                    blur_sprite.size = main_sprite.size;
                }

                if free_space.w > free_space.h {
                    blur_sprites[0].size.w = free_space.w * 0.5;
                    blur_sprites[0].set_sub_rect(Rect::new(
                        0,
                        0,
                        (free_space.w * 0.5) as _,
                        height,
                    ));

                    blur_sprites[1].position.x = display_size.w as f32 - free_space.w * 0.5;
                    blur_sprites[1].size.w = free_space.w * 0.5;
                    blur_sprites[1].set_sub_rect(Rect::new(
                        texture.size().w as i32 - (free_space.w * 0.5) as i32,
                        0,
                        (free_space.w * 0.5) as _,
                        height,
                    ));
                } else {
                    blur_sprites[0].size.h = free_space.h * 0.5;
                    blur_sprites[0].set_sub_rect(Rect::new(
                        0,
                        0,
                        width,
                        (free_space.h * 0.5) as i32,
                    ));

                    blur_sprites[1].position.y = display_size.h as f32 - free_space.h * 0.5;
                    blur_sprites[1].size.h = free_space.h * 0.5;
                    blur_sprites[1].set_sub_rect(Rect::new(
                        0,
                        texture.size().h as i32 - (free_space.h * 0.5) as i32,
                        width,
                        (free_space.h * 0.5) as i32,
                    ));
                }
                background = Some(blur_sprites);
            }
        }

        let date = preloaded_slide.details.date.map(|date| {
            date.date_naive()
                .format_localized(&config.slideshow.date.format, config.slideshow.date.locale)
                .to_string()
        });
        let text = [preloaded_slide.details.city, date]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let text = if text.is_empty() {
            None
        } else {
            Some(text.join("\n"))
        };

        let text = text
            .map(|text| TextWithBackground::create(graphics, text))
            .transpose()?;

        Ok(Slide {
            main_sprite,
            background,
            text,
        })
    }

    fn set_opacity(&mut self, alpha: f32) {
        for sprite in self.background.iter_mut().flatten() {
            sprite.opacity = alpha;
        }
        self.main_sprite.opacity = alpha;
        if let Some(text) = &mut self.text {
            text.set_opacity(alpha);
        };
    }

    fn apply(&mut self, properties: SlideProperties) {
        self.set_opacity(properties.global_opacity);
        self.main_sprite
            .set_sub_center_size(0.5.into(), (properties.zoom * 0.5).into());
    }
}

impl TextWithBackground {
    fn create(graphics: &mut Graphics, text: String) -> Result<Self> {
        let display_size = graphics.get_dimensions();
        let bottom_padding = 10f32;
        let bg_padding = 5f32;

        let container = {
            let container = graphics
                .create_text_container()
                .context("Cannot create text container")?;
            container.set_layout(LayoutJob {
                halign: epaint::emath::Align::Center,
                ..LayoutJob::single_section(
                    text,
                    TextFormat::simple(FontId::proportional(28.), Color32::WHITE),
                )
            });
            graphics.force_text_container_update(&container);
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
                    epaint::Rect::from_min_size(Pos2::ZERO, epaint::Vec2::new(dims.w, dims.h)),
                    10f32,
                    Color32::BLACK.linear_multiply(0.5),
                )
            };
            let mut shape = graphics.create_shape(rect.into(), None)?;
            shape.position = container.get_bounding_rect().position() - bg_padding;
            shape
        };
        Ok(Self {
            container,
            background: shape,
        })
    }

    fn set_opacity(&mut self, alpha: f32) {
        self.container.set_opacity(alpha);
        self.background.opacity_factor = alpha;
    }
}

impl Slides {
    pub fn should_load_next(&self) -> bool {
        match self {
            Slides::None => true,
            Slides::Single(slide) => slide.animation.is_finished(Instant::now()),
            Slides::Transitioning(_) => false,
        }
    }

    pub fn load_next(self, slide: Slide, config: &Conf) -> Self {
        match self {
            Slides::None => Self::to_single(
                slide,
                SlideProperties {
                    zoom: 0.9,
                    ..SlideProperties::default()
                },
                config,
                Instant::now(),
            ),
            Slides::Single(old)
            | Slides::Transitioning(TransitioningSlide { old: _, new: old }) => {
                let transition_duration = config.slideshow.transition_duration;
                let easing = glissade::Easing::QuarticInOut;
                let now = Instant::now();
                let old_properties = old.animation.get(now);
                let old = AnimatedSlide {
                    slide: old.slide,
                    animation: Box::new(
                        glissade::keyframes::from(old_properties.clone())
                            .ease_to(
                                SlideProperties {
                                    global_opacity: 0.,
                                    ..old_properties
                                },
                                transition_duration,
                                easing.clone(),
                            )
                            .run(now),
                    ),
                };
                let new = AnimatedSlide {
                    slide,
                    animation: Box::new(
                        glissade::keyframes::from(SlideProperties {
                            global_opacity: 0.,
                            zoom: 0.9,
                            ..Default::default()
                        })
                        .ease_to(
                            SlideProperties {
                                global_opacity: 1.,
                                zoom: 0.9,
                                ..Default::default()
                            },
                            transition_duration,
                            easing,
                        )
                        .run(now),
                    ),
                };

                Slides::Transitioning(TransitioningSlide { old, new })
            }
        }
    }

    pub fn update(mut self, config: &Conf) -> Self {
        let instant = Instant::now();
        match self {
            Slides::None => self,
            Slides::Single(ref mut slide) => {
                slide.update(instant);
                self
            }
            Slides::Transitioning(mut t) => {
                if t.is_finished(instant) {
                    Self::to_single(t.new.slide, t.new.animation.get(instant), config, instant)
                } else {
                    t.update(instant);
                    Slides::Transitioning(t)
                }
            }
        }
    }

    fn to_single(
        slide: Slide,
        current_properties: SlideProperties,
        config: &Conf,
        start: Instant,
    ) -> Self {
        let animation = glissade::keyframes::from(current_properties.clone())
            .ease_to(
                SlideProperties {
                    // zoom: 0.9,
                    ..Default::default()
                },
                config.slideshow.display_duration,
                glissade::Easing::CubicInOut,
            )
            .run(start);

        Self::Single(AnimatedSlide {
            slide,
            animation: Box::new(animation),
        })
    }
}

impl TransitioningSlide {
    fn is_finished(&self, instant: Instant) -> bool {
        self.old.animation.is_finished(instant) && self.new.animation.is_finished(instant)
    }

    fn update(&mut self, instant: Instant) {
        self.old.update(instant);
        self.new.update(instant);
    }
}

impl Drawable for TransitioningSlide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics.draw(&self.old)?;
        graphics.draw(&self.new)?;
        Ok(())
    }
}

impl Drawable for Slide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        for sprite in self.background.iter().flatten() {
            graphics.draw(sprite)?;
        }
        graphics.draw(&self.main_sprite)?;
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
            Slides::Single(slide) => graphics.draw(slide),
            Slides::Transitioning(transitioning_slide) => graphics.draw(transitioning_slide),
        }
    }
}

impl Drawable for TextWithBackground {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics.draw(&self.background)?;
        graphics.draw(&self.container)?;
        Ok(())
    }
}

impl Drawable for AnimatedSlide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        graphics.draw(&self.slide)
    }
}
