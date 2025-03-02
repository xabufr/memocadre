use std::time::Instant;

use anyhow::{Context, Result};
use epaint::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, Pos2, RectShape,
};
use vek::{Extent2, Rect, Vec2};

use crate::{
    application::slideshow::animated_properties::animated_properties,
    configuration::{AppConfiguration, Background, BlurBackground},
    gallery::ImageDetails,
    gl::texture::DetachedTexture,
    graphics::{Drawable, Graphics, ShapeContainer, SharedTexture2d, Sprite, TextContainer},
    worker::PreloadedSlide,
};

pub struct Slide {
    main_sprite: Sprite,
    background: Option<[Sprite; 2]>,
    text: Option<TextWithBackground>,
}

pub struct AnimatedSlide {
    pub slide: Slide,
    pub animation: AnimatedSlideProperties,
}

pub struct TextWithBackground {
    container: TextContainer,
    background: ShapeContainer,
    bg_padding: f32,
}

animated_properties!(SlideProperties {
    global_opacity: f32 = 1.0,
    zoom: f32 = 1.0,
    text_position: [f32; 2] = [0.0, 0.0],
});

const BG_PADDING: f32 = 5.0;
const TEXT_CORNER_RADIUS: f32 = 10.0;
const BACKGROUND_BLUR_ALPHA: f32 = 0.5;

impl AnimatedSlide {
    pub fn update(&mut self, instant: Instant) {
        let properties = self.animation.to_slide_properties(instant);
        self.slide.apply(properties);
    }
}

impl Slide {
    pub fn create(
        preloaded_slide: PreloadedSlide,
        graphics: &mut Graphics,
        config: &AppConfiguration,
    ) -> Result<Self> {
        let texture = SharedTexture2d::new(graphics.texture_from_detached(preloaded_slide.texture));
        let main_sprite = Self::create_main_sprite(graphics, &texture)?;

        let background = Self::create_blurred_background(
            graphics,
            preloaded_slide.blurred_texture,
            config,
            &main_sprite,
        )?;

        let text = Self::create_text(graphics, &preloaded_slide.details, config)?;

        Ok(Slide {
            main_sprite,
            background,
            text,
        })
    }

    fn create_main_sprite(graphics: &mut Graphics, texture: &SharedTexture2d) -> Result<Sprite> {
        let mut main_sprite = Sprite::new(SharedTexture2d::clone(texture));
        let display_size = graphics.get_dimensions();
        main_sprite.resize_respecting_ratio(display_size);

        let free_space = display_size.as_() - main_sprite.size;
        main_sprite.position = Vec2::from(free_space * 0.5).round();
        Ok(main_sprite)
    }

    fn create_blurred_background(
        graphics: &mut Graphics,
        blurred_texture: DetachedTexture,
        config: &AppConfiguration,
        main_sprite: &Sprite,
    ) -> Result<Option<[Sprite; 2]>> {
        if let Background::Blur(BlurBackground { min_free_space }) = config.slideshow.background {
            let display_size = graphics.get_dimensions();
            let free_space = display_size.as_::<f32>() - main_sprite.size;
            if free_space.reduce_partial_max() > min_free_space as f32 {
                let texture_blur = graphics.texture_from_detached(blurred_texture);
                let texture_blur = SharedTexture2d::new(texture_blur);

                let background_sprites =
                    Self::calculate_background_sprites(main_sprite, &texture_blur, display_size);
                return Ok(Some(background_sprites));
            }
        }
        Ok(None)
    }

    fn calculate_background_sprites(
        main_sprite: &Sprite,
        texture_blur: &SharedTexture2d,
        display_size: Extent2<u32>,
    ) -> [Sprite; 2] {
        let mut blur_sprites = [
            Sprite::new(SharedTexture2d::clone(texture_blur)),
            Sprite::new(SharedTexture2d::clone(texture_blur)),
        ];

        for blur_sprite in blur_sprites.iter_mut() {
            blur_sprite.size = main_sprite.size;
        }

        let (width, height) = display_size.as_::<i32>().into_tuple();
        let free_space = display_size.as_::<f32>() - main_sprite.size;
        if free_space.w > free_space.h {
            blur_sprites[0].size.w = main_sprite.position.x;
            blur_sprites[0].set_sub_rect(Rect::new(0, 0, main_sprite.position.x as _, height));

            blur_sprites[1].position.x = main_sprite.position.x + main_sprite.size.w;
            blur_sprites[1].size.w = display_size.w as f32 - blur_sprites[1].position.x;
            blur_sprites[1].set_sub_rect(Rect::new(
                texture_blur.size().w as i32 - main_sprite.position.x as i32,
                0,
                main_sprite.position.x as _,
                height,
            ));
        } else {
            blur_sprites[0].size.h = main_sprite.position.y;
            blur_sprites[0].set_sub_rect(Rect::new(0, 0, width, main_sprite.position.y as i32));

            blur_sprites[1].position.y = main_sprite.position.y + main_sprite.size.h;
            blur_sprites[1].size.h = main_sprite.position.y;
            blur_sprites[1].set_sub_rect(Rect::new(
                0,
                texture_blur.size().h as i32 - main_sprite.position.y as i32,
                width,
                main_sprite.position.y as i32,
            ));
        }
        blur_sprites
    }

    fn create_text(
        graphics: &mut Graphics,
        details: &ImageDetails,
        config: &AppConfiguration,
    ) -> Result<Option<TextWithBackground>> {
        if !config.slideshow.caption.enabled {
            return Ok(None);
        }

        let date = details.date.map(|date| {
            date.date_naive()
                .format_localized(
                    &config.slideshow.caption.date_format.format,
                    config.slideshow.caption.date_format.locale,
                )
                .to_string()
        });
        let text = [details.city.clone(), date]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if text.is_empty() {
            return Ok(None);
        }

        let text = text.join("\n");
        TextWithBackground::create(graphics, text, config.slideshow.caption.font_size)
            .map(Some)
            .context("Failed to create text for slide")
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

    pub fn get_text(&self) -> Option<&TextWithBackground> {
        self.text.as_ref()
    }

    pub fn apply(&mut self, properties: SlideProperties) {
        self.set_opacity(properties.global_opacity);
        self.main_sprite
            .set_sub_center_size(0.5.into(), (properties.zoom * 0.5).into());
        if let Some(text) = self.text.as_mut() {
            text.set_position(properties.text_position.into());
        }
    }
}

impl TextWithBackground {
    // TODO Test me !
    fn create(graphics: &mut Graphics, text: String, font_size: f32) -> Result<Self> {
        let container = {
            let container = graphics
                .create_text_container()
                .context("Cannot create text container")?;
            container.set_layout(LayoutJob {
                halign: epaint::emath::Align::Center,
                ..LayoutJob::single_section(
                    text,
                    TextFormat::simple(FontId::proportional(font_size), Color32::WHITE),
                )
            });
            graphics.force_text_container_update(&container);
            container
        };
        let shape = {
            let dims = container.get_dimensions() + BG_PADDING * 2.;
            let rect = RectShape {
                blur_width: BG_PADDING,
                ..RectShape::filled(
                    epaint::Rect::from_min_size(Pos2::ZERO, epaint::Vec2::new(dims.w, dims.h)),
                    TEXT_CORNER_RADIUS,
                    Color32::BLACK.linear_multiply(BACKGROUND_BLUR_ALPHA),
                )
            };
            graphics.create_shape(rect.into(), None)?
        };
        Ok(Self {
            container,
            background: shape,
            bg_padding: BG_PADDING,
        })
    }

    fn set_opacity(&mut self, alpha: f32) {
        self.container.set_opacity(alpha);
        self.background.set_opacity(alpha);
    }

    fn set_position(&mut self, position: Vec2<f32>) {
        let c_pos = self.container.get_position();
        // Text origin may not be at the top left corner
        let offset = c_pos - self.container.get_bounding_rect().position();
        self.container
            .set_position(position + offset + self.bg_padding);
        self.background.set_position(position);
    }

    pub fn size(&self) -> Extent2<f32> {
        self.container.get_dimensions() + self.bg_padding * 2_f32
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

    use chrono::{Locale, NaiveDate, Utc};
    use googletest::{
        assert_pred, expect_pred, expect_that, gtest,
        matchers::matches_pattern,
        prelude::{approx_eq, eq},
    };
    use vek::{Extent2, Vec2};

    use super::{AppConfiguration, Background, PreloadedSlide, Slide};
    use crate::{
        configuration::{BlurBackground, OrientationName},
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

        let mut config = AppConfiguration::mock();
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

        let mut config = AppConfiguration::mock();
        config.slideshow.background = Background::Blur(BlurBackground { min_free_space: 50 });
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
        let mut config = AppConfiguration::mock();
        config.slideshow.background = Background::Blur(BlurBackground { min_free_space: 50 });
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

        let config = AppConfiguration::mock();
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

        let mut config = AppConfiguration::mock();
        config.slideshow.caption.date_format.locale = Locale::fr_FR;
        config.slideshow.caption.date_format.format = "%A %e %B %Y".into();
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
