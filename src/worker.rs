use std::sync::{
    mpsc::{Receiver, SyncSender},
    Arc, RwLock, Weak,
};

use anyhow::{Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use log::error;
use thread_priority::{set_current_thread_priority, ThreadPriority};
use vek::Extent2;

use crate::{
    configuration::Conf,
    gallery::{build_sources, ImageWithDetails},
};

type Message = ImageWithDetails;
pub struct Worker {
    worker_impl: Weak<WorkerImpl>,
    recv: Receiver<Message>,
}

struct WorkerImpl {
    send: SyncSender<Message>,
    ideal_max_size: RwLock<Extent2<u32>>,
    config: Arc<Conf>,
}

impl Worker {
    pub fn new(config: Arc<Conf>, ideal_max_size: Extent2<u32>) -> Self {
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
            worker_impl
                .work()
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
    fn work(&self) -> Result<()> {
        if let Err(err) = set_current_thread_priority(ThreadPriority::Min) {
            error!("Cannot change worker thread priority to minimal: {:?}", err);
        }
        let mut source = build_sources(&self.config.sources).context("Cannot build source")?;
        loop {
            let mut img_with_details = source.get_next_image()?;
            img_with_details.image = self.resize_image_if_necessay(img_with_details.image);
            self.send
                .send(img_with_details)
                .context("While sending next image to display thread")?;
        }
    }

    fn resize_image_if_necessay(&self, image: DynamicImage) -> DynamicImage {
        let image_dims: Extent2<u32> = image.dimensions().into();
        let ideal_size = {
            let r = self
                .ideal_max_size
                .read()
                .expect("Cannot read ideal_max_size");
            r.clone()
        };
        let should_resize = image_dims.cmpgt(&ideal_size).reduce_or();
        return if should_resize {
            image.resize(ideal_size.w, ideal_size.h, FilterType::Lanczos3)
        } else {
            image
        };
    }
}
