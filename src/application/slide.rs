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

pub enum Slideshow {
    None,
    Single(AnimatedSlide),
    Transitioning(TransitioningSlide),
}

pub struct TransitioningSlide {
    prev: AnimatedSlide,
    next: AnimatedSlide,
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
    // TODO Test me !
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
            shape.set_position(container.get_bounding_rect().position() - bg_padding);
            shape
        };
        Ok(Self {
            container,
            background: shape,
        })
    }

    fn set_opacity(&mut self, alpha: f32) {
        self.container.set_opacity(alpha);
        self.background.set_opacity(alpha);
    }
}

impl Slideshow {
    // TODO: Take instant as argument
    pub fn should_load_next(&self) -> bool {
        match self {
            Slideshow::None => true,
            Slideshow::Single(slide) => slide.animation.is_finished(Instant::now()),
            Slideshow::Transitioning(_) => false,
        }
    }

    // TODO: Take instant as argument
    pub fn load_next(&mut self, slide: Slide, config: &Conf) {
        let mut old_self = Self::None;
        std::mem::swap(self, &mut old_self);
        match old_self {
            Slideshow::None => {
                *self = Self::to_single(
                    slide,
                    SlideProperties {
                        zoom: 0.9,
                        ..SlideProperties::default()
                    },
                    config,
                    Instant::now(),
                )
            }
            Slideshow::Single(old)
            | Slideshow::Transitioning(TransitioningSlide { prev: _, next: old }) => {
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
                        })
                        .ease_to(
                            SlideProperties {
                                global_opacity: 1.,
                                zoom: 0.9,
                            },
                            transition_duration,
                            easing,
                        )
                        .run(now),
                    ),
                };

                *self = Slideshow::Transitioning(TransitioningSlide {
                    prev: old,
                    next: new,
                })
            }
        }
    }

    // TODO: Take instant as argument
    // TODO: Test me !
    pub fn update(&mut self, config: &Conf) {
        let mut old_self = Self::None;
        std::mem::swap(self, &mut old_self);
        let instant = Instant::now();
        match old_self {
            Slideshow::None => (),
            Slideshow::Single(ref mut slide) => {
                slide.update(instant);
                *self = old_self
            }
            Slideshow::Transitioning(mut t) => {
                if t.is_finished(instant) {
                    *self = Self::to_single(
                        t.next.slide,
                        t.next.animation.get(instant),
                        config,
                        instant,
                    );
                } else {
                    t.update(instant);
                    *self = Slideshow::Transitioning(t);
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

impl Drawable for Slide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        for sprite in self.background.iter().flatten() {
            sprite.draw(graphics)?;
        }
        self.main_sprite.draw(graphics)?;
        if let Some(text) = &self.text {
            text.draw(graphics)?;
        }
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

impl Drawable for TextWithBackground {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        self.background.draw(graphics)?;
        self.container.draw(graphics)?;
        Ok(())
    }
}

impl Drawable for AnimatedSlide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        self.slide.draw(graphics)
    }
}

// Test module
#[cfg(test)]
mod test {
    use std::rc::Rc;

    use chrono::{DateTime, Locale, NaiveDate, Utc};
    use googletest::{
        assert_pred, expect_pred, expect_that, gtest,
        matchers::matches_pattern,
        prelude::{approx_eq, eq},
    };
    use vek::{Extent2, Vec2};

    use super::{Background, Conf, PreloadedSlide, Slide};
    use crate::{
        configuration::OrientationName,
        gallery::ImageDetails,
        gl::{texture::DetachedTexture, wrapper::mocked_gl, GlContext},
        graphics::{Graphics, TextureRegion},
    };

    fn preloaded_slide(size: Extent2<u32>) -> PreloadedSlide {
        PreloadedSlide {
            details: ImageDetails {
                city: None,
                date: None,
                people: Default::default(),
            },
            texture: DetachedTexture::mock(size),
            blurred_texture: DetachedTexture::mock(size),
        }
    }

    #[gtest]
    fn test_simple_slide_creation() {
        let gl = mocked_gl();
        let gl = Rc::new(GlContext::mocked(gl));
        let mut graphics = Graphics::new(gl.clone(), OrientationName::Angle0).unwrap();

        let mut config = Conf::mock();
        config.slideshow.background = Background::Black;
        let preloaded_slide = preloaded_slide((100, 100).into());

        let slide = Slide::create(preloaded_slide, &mut graphics, &config).unwrap();
        expect_pred!(slide.background.is_none());
        expect_that!(
            slide.main_sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(600.),
                h: approx_eq(600.),
            })
        );
        expect_that!(
            slide.main_sprite.position,
            matches_pattern!(Vec2 {
                x: approx_eq(100.),
                y: approx_eq(0.),
            })
        );
        expect_pred!(slide.text.is_none());
    }

    #[gtest]
    fn test_slide_with_background_sides() {
        let gl = mocked_gl();
        let gl = Rc::new(GlContext::mocked(gl));
        let mut graphics = Graphics::new(gl.clone(), OrientationName::Angle0).unwrap();

        let mut config = Conf::mock();
        config.slideshow.background = Background::Burr { min_free_space: 50 };
        let preloaded_slide = preloaded_slide((400, 600).into());

        let slide = Slide::create(preloaded_slide, &mut graphics, &config).unwrap();
        expect_that!(
            slide.main_sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(400.),
                h: approx_eq(600.),
            })
        );
        expect_that!(
            slide.main_sprite.position,
            matches_pattern!(Vec2 {
                x: approx_eq(200.),
                y: approx_eq(0.),
            })
        );
        assert_pred!(slide.background.is_some());
        let background = slide.background.as_ref().unwrap();
        for i in 0..2 {
            expect_that!(
                background[i].size,
                matches_pattern!(Extent2 {
                    w: approx_eq(200.),
                    h: approx_eq(600.),
                })
            );
        }
        // verify background position
        expect_that!(
            background[0].position,
            matches_pattern!(Vec2 {
                x: approx_eq(0.),
                y: approx_eq(0.),
            })
        );
        expect_that!(
            background[1].position,
            matches_pattern!(Vec2 {
                x: approx_eq(600.),
                y: approx_eq(0.),
            })
        );
        // verify background sub_rect
        expect_that!(
            background[0].get_sub_center_size(),
            matches_pattern!(TextureRegion {
                uv_center: matches_pattern!(Vec2 {
                    x: approx_eq(0.25),
                    y: approx_eq(0.5)
                }),
                uv_size: matches_pattern!(Extent2 {
                    w: approx_eq(0.25),
                    h: approx_eq(0.5)
                }),
            })
        );
        expect_that!(
            background[1].get_sub_center_size(),
            matches_pattern!(TextureRegion {
                uv_center: matches_pattern!(Vec2 {
                    x: approx_eq(0.75),
                    y: approx_eq(0.5)
                }),
                uv_size: matches_pattern!(Extent2 {
                    w: approx_eq(0.25),
                    h: approx_eq(0.5)
                }),
            })
        );
    }

    #[gtest]
    fn test_slide_with_background_top_bottom() {
        let gl = mocked_gl();
        let gl = Rc::new(GlContext::mocked(gl));
        let mut graphics = Graphics::new(gl.clone(), OrientationName::Angle0).unwrap();
        let mut config = Conf::mock();
        config.slideshow.background = Background::Burr { min_free_space: 50 };
        let preloaded_slide = preloaded_slide((800, 400).into());

        let slide = Slide::create(preloaded_slide, &mut graphics, &config).unwrap();
        expect_that!(
            slide.main_sprite.size,
            matches_pattern!(Extent2 {
                w: approx_eq(800.),
                h: approx_eq(400.),
            })
        );
        expect_that!(
            slide.main_sprite.position,
            matches_pattern!(Vec2 {
                x: approx_eq(0.),
                y: approx_eq(100.),
            })
        );
        assert_pred!(slide.background.is_some());
        let background = slide.background.as_ref().unwrap();
        for i in 0..2 {
            expect_that!(
                background[i].size,
                matches_pattern!(Extent2 {
                    w: approx_eq(800.),
                    h: approx_eq(100.),
                })
            );
        }
        // verify background position
        expect_that!(
            background[0].position,
            matches_pattern!(Vec2 {
                x: approx_eq(0.),
                y: approx_eq(0.),
            })
        );
        expect_that!(
            background[1].position,
            matches_pattern!(Vec2 {
                x: approx_eq(0.),
                y: approx_eq(500.),
            })
        );
        // verify background sub_rect
        expect_that!(
            background[0].get_sub_center_size(),
            matches_pattern!(TextureRegion {
                uv_center: matches_pattern!(Vec2 {
                    x: approx_eq(0.5),
                    y: approx_eq(0.125)
                }),
                uv_size: matches_pattern!(Extent2 {
                    w: approx_eq(0.5),
                    h: approx_eq(0.125)
                }),
            })
        );
        expect_that!(
            background[1].get_sub_center_size(),
            matches_pattern!(TextureRegion {
                uv_center: matches_pattern!(Vec2 {
                    x: approx_eq(0.5),
                    y: approx_eq(0.875)
                }),
                uv_size: matches_pattern!(Extent2 {
                    w: approx_eq(0.5),
                    h: approx_eq(0.125)
                }),
            })
        );
    }

    #[gtest]
    fn test_slide_text() {
        let gl = mocked_gl();
        let gl = Rc::new(GlContext::mocked(gl));
        let mut graphics = Graphics::new(gl.clone(), OrientationName::Angle0).unwrap();

        let config = Conf::mock();
        let mut preloaded_slide = preloaded_slide((800, 600).into());
        preloaded_slide.details.city = Some("A wonderfull city".into());

        let slide = Slide::create(preloaded_slide, &mut graphics, &config).unwrap();
        assert_pred!(slide.text.is_some());
        let text = slide.text.as_ref().unwrap();
        let galley = text.container.galley().unwrap();
        expect_that!(galley.text(), eq("A wonderfull city"));
    }

    #[gtest]
    fn test_slide_text_date() {
        let gl = mocked_gl();
        let gl = Rc::new(GlContext::mocked(gl));
        let mut graphics = Graphics::new(gl.clone(), OrientationName::Angle0).unwrap();

        let mut config = Conf::mock();
        config.slideshow.date.locale = Locale::fr_FR;
        config.slideshow.date.format = "%A %e %B %Y".into();
        let mut preloaded_slide = preloaded_slide((800, 600).into());
        let date = NaiveDate::from_ymd_opt(2025, 01, 25)
            .unwrap()
            .and_hms_opt(12, 30, 50)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        preloaded_slide.details.date = Some(date);

        let slide = Slide::create(preloaded_slide, &mut graphics, &config).unwrap();
        assert_pred!(slide.text.is_some());
        let text = slide.text.as_ref().unwrap();
        let galley = text.container.galley().unwrap();
        expect_that!(galley.text(), eq("samedi 25 janvier 2025"));
    }
}
