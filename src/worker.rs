use std::{
    rc::Rc,
    sync::mpsc::{Receiver, SyncSender},
    time::Duration,
};

use anyhow::{Context, Result};
use backon::{BlockingRetryable, ExponentialBuilder};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use log::error;
use thread_priority::{set_current_thread_priority, ThreadPriority};
use tokio::sync::watch;
use vek::Extent2;

use crate::{
    configuration::{ImageFilter, Settings, Source},
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
    ideal_max_size_sender: watch::Sender<Extent2<u32>>,
    recv: Receiver<Message>,
}

struct WorkerImpl {
    send: SyncSender<Message>,
    ideal_max_size: watch::Receiver<Extent2<u32>>,
    config: Settings,
    config_watch: watch::Receiver<Settings>,
    sources: Vec<Source>,
}

impl Worker {
    pub fn new(
        mut config_watch: watch::Receiver<Settings>,
        ideal_max_size: Extent2<u32>,
        gl: FutureGlThreadContext,
        sources: Vec<Source>,
    ) -> Self {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let config = config_watch.borrow_and_update().clone();
        let (ideal_max_size_sender, ideal_max_size_receiver) = watch::channel(ideal_max_size);
        let mut worker_impl = WorkerImpl {
            send,
            ideal_max_size: ideal_max_size_receiver,
            config,
            config_watch,
            sources,
        };
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
            ideal_max_size_sender,
            recv,
        }
    }

    pub fn set_ideal_max_size(&self, size: Extent2<u32>) {
        self.ideal_max_size_sender.send_replace(size);
    }

    pub fn recv(&self) -> &Receiver<Message> {
        &self.recv
    }
}
impl WorkerImpl {
    fn work(&mut self, gl: &Rc<GlContext>, blurr: &ImageBlurr) -> Result<()> {
        if let Err(err) = set_current_thread_priority(ThreadPriority::Min) {
            error!("Cannot change worker thread priority to minimal: {:?}", err);
        }
        let mut source = build_sources(&self.sources).context("Cannot build source")?;
        loop {
            if let Ok(true) = self.config_watch.has_changed() {
                self.config = self.config_watch.borrow_and_update().clone();
            }
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
            .blur(self.config.blur_options.clone().into(), &texture)
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
        let ideal_size = *self.ideal_max_size.borrow();
        let should_resize = image_dims.cmpgt(&ideal_size).reduce_or();
        if should_resize {
            let filter = self.config.downscaled_image_filter;
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
