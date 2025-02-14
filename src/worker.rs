use std::{
    rc::Rc,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, RwLock, Weak,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use backon::{BlockingRetryable, ExponentialBuilder};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use log::error;
use thread_priority::{set_current_thread_priority, ThreadPriority};
use vek::Extent2;

use crate::{
    configuration::{AppConfiguration, ImageFilter},
    gallery::{build_sources, Gallery, ImageDetails},
    gl::{
        texture::{DetachedTexture, Texture},
        FutureGlThreadContext, GlContext,
    },
    graphics::ImageBlurr,
};

type Message = PreloadedSlide;

pub struct PreloadedSlide {
    pub details: ImageDetails,
    pub texture: DetachedTexture,
    pub blurred_texture: DetachedTexture,
}

pub struct Worker {
    worker_impl: Weak<WorkerImpl>,
    recv: Receiver<Message>,
}

struct WorkerImpl {
    send: SyncSender<Message>,
    ideal_max_size: RwLock<Extent2<u32>>,
    config: Arc<AppConfiguration>,
}

impl Worker {
    pub fn new(
        config: Arc<AppConfiguration>,
        ideal_max_size: Extent2<u32>,
        gl: FutureGlThreadContext,
    ) -> Self {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let worker_impl = Arc::new({
            WorkerImpl {
                send,
                ideal_max_size: RwLock::new(ideal_max_size),
                config,
            }
        });
        let worker_impl_weak = Arc::downgrade(&worker_impl);
        std::thread::spawn(move || {
            let gl = gl
                .activate()
                .expect("Cannot make worker thread context current");
            let blurr =
                crate::graphics::ImageBlurr::new(gl.clone()).expect("Cannot create ImageBlurr");
            worker_impl
                .work(&gl, &blurr)
                .expect("Worker encountered an error, abort");
        });
        Worker {
            worker_impl: worker_impl_weak,
            recv,
        }
    }

    pub fn set_ideal_max_size(&self, size: Extent2<u32>) {
        if let Some(worker_impl) = self.worker_impl.upgrade() {
            let mut w = worker_impl
                .ideal_max_size
                .write()
                .expect("Cannot lock worker ideal_max_size");
            *w = size;
        }
    }

    pub fn recv(&self) -> &Receiver<Message> {
        &self.recv
    }
}
impl WorkerImpl {
    fn work(&self, gl: &Rc<GlContext>, blurr: &ImageBlurr) -> Result<()> {
        if let Err(err) = set_current_thread_priority(ThreadPriority::Min) {
            error!("Cannot change worker thread priority to minimal: {:?}", err);
        }
        let mut source = build_sources(&self.config.sources).context("Cannot build source")?;
        loop {
            let msg = (|| self.get_next(&mut *source, gl, blurr))
                .retry(
                    ExponentialBuilder::default()
                        .with_max_delay(Duration::from_secs(10))
                        .with_max_times(10),
                )
                .call()?;
            self.send
                .send(msg)
                .context("While sending next image to display thread")?;
        }
    }

    fn get_next(
        &self,
        source: &mut dyn Gallery,
        gl: &Rc<GlContext>,
        blurr: &ImageBlurr,
    ) -> Result<PreloadedSlide> {
        let mut img_with_details = source.get_next_image()?;
        img_with_details.image = self.resize_image_if_necessay(img_with_details.image);
        let texture = Texture::new_from_image(gl.clone(), &img_with_details.image).unwrap();
        let blurred_texture = blurr
            .blur(self.config.slideshow.blur_options.clone().into(), &texture)
            .unwrap();
        unsafe { gl.finish() };
        let msg = PreloadedSlide {
            details: img_with_details.details,
            texture: texture.detach(),
            blurred_texture: blurred_texture.detach(),
        };
        Ok(msg)
    }

    fn resize_image_if_necessay(&self, image: DynamicImage) -> DynamicImage {
        let image_dims: Extent2<u32> = image.dimensions().into();
        let ideal_size = {
            let r = self
                .ideal_max_size
                .read()
                .expect("Cannot read ideal_max_size");
            *r
        };
        let should_resize = image_dims.cmpgt(&ideal_size).reduce_or();
        if should_resize {
            let filter = self.config.slideshow.downscaled_image_filter;
            image.resize(ideal_size.w, ideal_size.h, filter.into())
        } else {
            image
        }
    }
}

impl From<ImageFilter> for FilterType {
    fn from(f: ImageFilter) -> Self {
        match f {
            ImageFilter::Nearest => FilterType::Nearest,
            ImageFilter::Triangle => FilterType::Triangle,
            ImageFilter::CatmullRom => FilterType::CatmullRom,
            ImageFilter::Gaussian => FilterType::Gaussian,
            ImageFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}
