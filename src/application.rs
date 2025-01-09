use glam::Vec2;
use glissade::Keyframes;
use glium::{backend::Facade, CapabilitiesSource, Surface};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use log::debug;
use reqwest::header::X_FRAME_OPTIONS;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::support::{ApplicationContext, State};

use crate::graphics::{ImageBlurr, ImageDrawer, SharedTexture2d, Sprite};

struct Application {
    image_drawer: ImageDrawer,
    image_blurr: ImageBlurr,
    current_slide: Option<Slide>,
    next_slide: Option<TransitionningSlide>,
    image_display_start: Instant,
    recv: Receiver<DynamicImage>,
    counter: FPSCounter,
    _worker: JoinHandle<()>,
}

struct Slide {
    sprites: Vec<Sprite>,
}

struct TransitionningSlide {
    slide: Slide,
    animation: Box<dyn glissade::Animated<f32, Instant>>,
}

struct FPSCounter {
    last_fps: u32,
    last_instant: Instant,
    frames: u32,
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
    fn new(display: &glium::Display<glutin::surface::WindowSurface>) -> Self {
        debug!(
            "Starting with {}",
            display.get_context().get_opengl_version_string(),
        );
        let (send, recv) = sync_channel(1);
        let worker = thread::spawn(move || {
            use crate::galery::{Galery, ImmichGalery};
            let mut immich = ImmichGalery::new(
                "***REMOVED***",
                "***REMOVED***",
            );
            loop {
                let img = immich.get_next_image();
                send.send(img).unwrap();
            }
        });

        Self {
            image_drawer: ImageDrawer::new(display),
            image_blurr: ImageBlurr::new(display),
            current_slide: None,
            next_slide: None,
            image_display_start: Instant::now(),
            recv,
            counter: FPSCounter::new(),
            _worker: worker,
        }
    }

    fn draw_frame(&mut self, display: &glium::Display<glutin::surface::WindowSurface>) {
        let mut frame = display.draw();

        if self.current_slide.is_none()
            || (self.image_display_start.elapsed() >= Duration::from_secs_f32(3.)
                && self.next_slide.is_none())
        {
            match self.recv.try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(image) => {
                    let slide = self.load_next_frame(display, image);
                    self.image_display_start = Instant::now();
                    if self.current_slide.is_none() {
                        self.current_slide = Some(slide);
                    } else {
                        let animation = glissade::keyframes::from(0. as f32)
                            .ease_to(
                                1.,
                                Duration::from_secs_f32(1.),
                                glissade::Easing::QuarticInOut,
                            )
                            .run(self.image_display_start);
                        self.next_slide = Some(TransitionningSlide {
                            slide,
                            animation: Box::new(animation),
                        });
                    }
                }
            }
        }

        let frame_time = Instant::now();
        self.counter.count_frame();

        frame.clear_color(0.0, 0.0, 0.0, 0.0);
        if let Some(slide) = &self.current_slide {
            slide.draw(&mut frame, &self.image_drawer);
        }
        if let Some(next_slide) = &mut self.next_slide {
            let alpha = next_slide.animation.get(frame_time);
            for s in next_slide.slide.sprites.iter_mut() {
                s.opacity = alpha;
            }
            next_slide.slide.draw(&mut frame, &self.image_drawer);
            if next_slide.animation.is_finished(frame_time) {
                self.current_slide = self.next_slide.take().map(|a| a.slide);
            }
        }
        frame.finish().unwrap();
    }
}
impl Application {
    fn load_next_frame(
        &self,
        display: &glium::Display<glutin::surface::WindowSurface>,
        image: DynamicImage,
    ) -> Slide {
        let image = soft_resize_image_if_necessary(display, image, FilterType::Lanczos3);

        let texture = SharedTexture2d::new(image_to_texture(display, image));

        let mut sprite = Sprite::new(SharedTexture2d::clone(&texture));
        let (width, height) = display.get_framebuffer_dimensions();
        let display_size = Vec2::new(width as _, height as _);
        sprite.resize_respecting_ratio(display_size);

        let free_space = display_size - sprite.size;
        sprite.position = free_space * 0.5;

        let mut sprites = vec![];
        if free_space.max_element() > 50.0 {
            let texture_blur = SharedTexture2d::new(self.image_blurr.blur(display, &texture));
            let mut blur_sprites = [
                Sprite::new(SharedTexture2d::clone(&texture_blur)),
                Sprite::new(texture_blur),
            ];

            for blur_sprite in blur_sprites.iter_mut() {
                blur_sprite.size = sprite.size;
            }

            if free_space.x > 50. {
                blur_sprites[1].position.x = display_size.x - blur_sprites[1].size.x;
            } else {
                blur_sprites[1].position.y = display_size.y - blur_sprites[1].size.y;
            }
            sprites.extend(blur_sprites.into_iter());
        }
        sprites.push(sprite);

        return Slide { sprites };
    }
}

impl Slide {
    pub fn draw(&self, surface: &mut impl Surface, image_drawer: &ImageDrawer) {
        for sprite in self.sprites.iter() {
            image_drawer.draw_sprite(surface, sprite);
        }
    }
}

fn image_to_texture(
    display: &glium::Display<glutin::surface::WindowSurface>,
    image: DynamicImage,
) -> glium::Texture2d {
    let dims = image.dimensions();
    glium::Texture2d::new(
        display,
        glium::texture::RawImage2d::from_raw_rgb(image.into_rgb8().into_raw(), dims),
    )
    .unwrap()
}

fn soft_resize_image_if_necessary(
    display: &glium::Display<glutin::surface::WindowSurface>,
    image: DynamicImage,
    filter: FilterType,
) -> DynamicImage {
    let (width, height) = image.dimensions();
    let max = display.get_context().get_capabilities().max_texture_size as u32;
    let image = if std::cmp::max(width, height) > max {
        image.resize(max, max, filter)
    } else {
        image
    };
    image
}

pub fn start() {
    State::<Application>::run_loop();
}
