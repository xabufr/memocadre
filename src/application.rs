use epaint::{text::LayoutJob, Color32, FontId};
use glam::{UVec2, Vec2};
use glissade::Keyframes;
use glium::{backend::Facade, CapabilitiesSource, Surface};
use glow::{Context, HasContext};
use image::{DynamicImage, GenericImageView};
use log::debug;
use std::{
    sync::mpsc::TryRecvError,
    time::{Duration, Instant},
};

use glyph_brush::{Section, Text};

use crate::{
    graphics::Texture,
    support::{self, ApplicationContext, State},
};
use crate::{
    graphics::{GlowImageBlurr, GlowImageDrawer},
    worker::Worker,
};

use crate::graphics::{
    // EpaintDisplay,
    // ImageBlurr,
    // ImageDrawer,
    SharedTexture2d,
    Sprite,
    // TextDisplay,
};

pub struct GlowApplication {
    image_drawer: GlowImageDrawer,
    image_blurr: GlowImageBlurr,
    current_slide: Option<Slide>,
    next_slide: Option<TransitionningSlide>,
    image_display_start: Instant,
    // recv: Receiver<DynamicImage>,
    counter: FPSCounter,
    // text_display: GlowTextDisplay,
    // _worker: JoinHandle<()>,
    worker: Worker,
}
// struct Application {
//     image_drawer: ImageDrawer,
//     image_blurr: ImageBlurr,
//     current_slide: Option<Slide>,
//     next_slide: Option<TransitionningSlide>,
//     image_display_start: Instant,
//     counter: FPSCounter,
//     text_display: TextDisplay,
//     epaint: EpaintDisplay,
//     worker: Worker,
// }

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
impl GlowApplication {
    pub fn new(gl: &Context) -> Self {
        let worker = Worker::new(Self::get_ideal_image_size(gl));
        worker.start();
        Self {
            current_slide: None,
            image_display_start: Instant::now(),
            image_drawer: GlowImageDrawer::new(gl),
            image_blurr: GlowImageBlurr::new(gl),
            // text_display: GlowTextDisplay::new(ctx),
            // recv,
            next_slide: None,
            counter: FPSCounter::new(),
            worker,
        }
    }
    pub fn draw(&mut self, gl: &Context) {
        if self.current_slide.is_none()
            || self.image_display_start.elapsed() >= Duration::from_secs_f32(0.2)
        {
            match self.worker.recv().try_recv() {
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
                Ok(image) => {
                    let slide = self.load_next_frame(gl, image);
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

        // frame.clear_color(0.0, 0.0, 0.0, 0.0);
        if let Some(slide) = &self.current_slide {
            slide.draw(gl, &self.image_drawer);
        }
        if let Some(next_slide) = &mut self.next_slide {
            let alpha = next_slide.animation.get(frame_time);
            for s in next_slide.slide.sprites.iter_mut() {
                s.opacity = alpha;
            }
            next_slide.slide.draw(gl, &self.image_drawer);
            if next_slide.animation.is_finished(frame_time) {
                self.current_slide = self.next_slide.take().map(|a| a.slide);
            }
        }
        // self.text_display.queue(
        //     Section::new()
        //         .add_text(
        //             Text::new(&format!(
        //                 "FPS: {} ({} frames)",
        //                 self.counter.last_fps, self.counter.frames
        //             ))
        //             .with_scale(28.)
        //             .with_color((1., 1., 1., 1.)),
        //         )
        //         .with_screen_position((100., 200.)),
        // );
        // self.text_display.update(gl);
        // self.text_display.draw(gl);
        // frame.finish().unwrap();
    }
}
// impl ApplicationContext for Application {
//     const WINDOW_TITLE: &'static str = "test";
//     fn new(display: &glium::Display<glutin::surface::WindowSurface>) -> Self {
//         debug!(
//             "Starting with {}",
//             display.get_context().get_opengl_version_string(),
//         );
//         let worker = Worker::new(Self::get_ideal_image_size(display));
//         worker.start();

//         Self {
//             image_drawer: ImageDrawer::new(display),
//             image_blurr: ImageBlurr::new(display),
//             current_slide: None,
//             next_slide: None,
//             image_display_start: Instant::now(),
//             counter: FPSCounter::new(),
//             text_display: TextDisplay::new(display),
//             epaint: EpaintDisplay::new(display),
//             worker,
//         }
//     }
//     fn resized(
//         &mut self,
//         display: &glium::Display<glutin::surface::WindowSurface>,
//         _width: u32,
//         _height: u32,
//     ) {
//         self.worker
//             .set_ideal_max_size(Self::get_ideal_image_size(display));
//     }

//     fn draw_frame(&mut self, display: &glium::Display<glutin::surface::WindowSurface>) {
//         let mut frame = display.draw();
//         self.epaint.begin_frame();

//         if self.current_slide.is_none()
//             || (self.image_display_start.elapsed() >= Duration::from_secs_f32(3.)
//                 && self.next_slide.is_none())
//         {
//             match self.worker.recv().try_recv() {
//                 Err(TryRecvError::Empty) => {}
//                 Err(TryRecvError::Disconnected) => {}
//                 Ok(image) => {
//                     let slide = self.load_next_frame(display, image);
//                     self.image_display_start = Instant::now();
//                     if self.current_slide.is_none() {
//                         self.current_slide = Some(slide);
//                     } else {
//                         let animation = glissade::keyframes::from(0. as f32)
//                             .ease_to(
//                                 1.,
//                                 Duration::from_secs_f32(1.),
//                                 glissade::Easing::QuarticInOut,
//                             )
//                             .run(self.image_display_start);
//                         self.next_slide = Some(TransitionningSlide {
//                             slide,
//                             animation: Box::new(animation),
//                         });
//                     }
//                 }
//             }
//         }

//         let frame_time = Instant::now();
//         self.counter.count_frame();

//         frame.clear_color(0.0, 0.0, 0.0, 0.0);
//         if let Some(slide) = &self.current_slide {
//             slide.draw(&mut frame, &self.image_drawer);
//         }
//         if let Some(next_slide) = &mut self.next_slide {
//             let alpha = next_slide.animation.get(frame_time);
//             for s in next_slide.slide.sprites.iter_mut() {
//                 s.opacity = alpha;
//             }
//             next_slide.slide.draw(&mut frame, &self.image_drawer);
//             if next_slide.animation.is_finished(frame_time) {
//                 self.current_slide = self.next_slide.take().map(|a| a.slide);
//             }
//         }
//         self.text_display.queue(
//             Section::new()
//                 .add_text(
//                     Text::new(&format!(
//                         "FPS: {} ({} frames)",
//                         self.counter.last_fps, self.counter.frames
//                     ))
//                     .with_scale(28.)
//                     .with_color((1., 1., 1., 1.)),
//                 )
//                 .with_screen_position((100., 200.)),
//         );
//         self.text_display.update(display);
//         self.text_display.draw(&mut frame);
//         self.epaint.add_text(
//             Vec2::new(100., 100.),
//             LayoutJob::simple_singleline(
//                 format!(
//                     "FPS: {} ({} frames)",
//                     self.counter.last_fps, self.counter.frames
//                 ),
//                 FontId::proportional(28.),
//                 Color32::DEBUG_COLOR,
//             ),
//         );
//         self.epaint.update(display);
//         self.epaint.draw_texts(&mut frame);
//         frame.finish().unwrap();
//     }
// }
// impl Application {
//     fn get_ideal_image_size(display: &glium::Display<glutin::surface::WindowSurface>) -> UVec2 {
//         let hw_max = display.get_context().get_capabilities().max_texture_size as u32;
//         let hw_max = UVec2::splat(hw_max);
//         let fb_dims: UVec2 = display.get_framebuffer_dimensions().into();

//         let ideal_size = fb_dims.min(hw_max);
//         return ideal_size;
//     }

//     fn load_next_frame(
//         &self,
//         display: &glium::Display<glutin::surface::WindowSurface>,
//         image: DynamicImage,
//     ) -> Slide {
//         let texture = SharedTexture2d::new(image_to_texture(display, image));

//         let mut sprite = Sprite::new(SharedTexture2d::clone(&texture));
//         let (width, height) = display.get_framebuffer_dimensions();
//         let display_size = Vec2::new(width as _, height as _);
//         sprite.resize_respecting_ratio(display_size);

//         let free_space = display_size - sprite.size;
//         sprite.position = free_space * 0.5;

//         let mut sprites = vec![];
//         if free_space.max_element() > 50.0 {
//             let texture_blur = SharedTexture2d::new(self.image_blurr.blur(display, &texture));
//             let mut blur_sprites = [
//                 Sprite::new(SharedTexture2d::clone(&texture_blur)),
//                 Sprite::new(texture_blur),
//             ];

//             for blur_sprite in blur_sprites.iter_mut() {
//                 blur_sprite.size = sprite.size;
//             }

//             if free_space.x > 50. {
//                 blur_sprites[1].position.x = display_size.x - blur_sprites[1].size.x;
//                 blur_sprites[0].texture_rect = Some(glium::Rect {
//                     left: 0,
//                     bottom: 0,
//                     width: (free_space.x * 0.5) as u32 + 2,
//                     height,
//                 });
//                 blur_sprites[1].texture_rect = Some(glium::Rect {
//                     left: width - (free_space.x * 0.5) as u32 - 2,
//                     bottom: 0,
//                     width: (free_space.x * 0.5) as u32 + 2,
//                     height,
//                 });
//             } else {
//                 blur_sprites[1].position.y = display_size.y - blur_sprites[1].size.y;
//                 blur_sprites[0].texture_rect = Some(glium::Rect {
//                     left: 0,
//                     bottom: 0,
//                     width,
//                     height: (free_space.y * 0.5) as u32 + 2,
//                 });
//                 blur_sprites[1].texture_rect = Some(glium::Rect {
//                     left: 0,
//                     bottom: height - (free_space.y * 0.5) as u32 - 2,
//                     width,
//                     height: (free_space.y * 0.5) as u32 + 2,
//                 });
//             }
//             sprites.extend(blur_sprites.into_iter());
//         }
//         sprites.push(sprite);

//         return Slide { sprites };
//     }
// }

impl Slide {
    pub fn draw(&self, gl: &Context, image_drawer: &GlowImageDrawer) {
        for sprite in self.sprites.iter() {
            image_drawer.draw_sprite(gl, sprite);
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

// pub fn start() {
//     let vars = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"];
//     let has_window_system = vars.into_iter().any(|v| std::env::var_os(v).is_some());
//     if has_window_system {
//         State::<Application>::run_loop();
//     } else {
//         support::start_gbm::<Application>();
//     }
// }
impl GlowApplication {
    fn get_ideal_image_size(gl: &Context) -> UVec2 {
        let hw_max = Texture::max_texture_size(gl);
        let hw_max = UVec2::splat(hw_max);
        let mut dims: [i32; 4] = [0; 4];
        unsafe {
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut dims);
        };
        let [_, _, width, height] = dims;

        let fb_dims = UVec2::new(width as _, height as _);

        let ideal_size = fb_dims.min(hw_max);
        return ideal_size;
    }

    fn load_next_frame(&self, gl: &Context, image: DynamicImage) -> Slide {
        let texture = Texture::new_from_image(gl, &image);

        let mut dims: [i32; 4] = [0; 4];
        unsafe {
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut dims);
        };
        let [_, _, width, height] = dims;

        let texture = SharedTexture2d::new(texture);
        let mut sprite = Sprite::new(texture.clone());
        let display_size = Vec2::new(width as _, height as _);
        sprite.resize_respecting_ratio(display_size);

        let free_space = display_size - sprite.size;
        sprite.position = free_space * 0.5;

        let mut sprites = vec![];
        if free_space.max_element() > 50.0 {
            let texture_blur = SharedTexture2d::new(self.image_blurr.blur(gl, &texture));
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
