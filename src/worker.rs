use anyhow::{Context, Result};
use std::sync::{
    mpsc::{Receiver, SyncSender},
    Arc, RwLock,
};

use image::{imageops::FilterType, DynamicImage, GenericImageView};
use log::error;
use thread_priority::{set_current_thread_priority, ThreadPriority};
use vek::Extent2;

use crate::{configuration::Conf, galery::ImageWithDetails};

type Message = ImageWithDetails;
pub struct Worker {
    worker_impl: Arc<WorkerImpl>,
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
        Worker { worker_impl, recv }
    }

    pub fn set_ideal_max_size(&self, size: Extent2<u32>) {
        let mut w = self
            .worker_impl
            .ideal_max_size
            .write()
            .expect("Cannot lock worker ideal_max_size");
        *w = size;
    }

    pub fn start(&self) {
        let worker_impl = self.worker_impl.clone();
        std::thread::spawn(move || {
            worker_impl
                .work()
                .expect("Worker encountered an error, abort");
        });
    }

    pub fn recv(&self) -> &Receiver<Message> {
        &self.recv
    }
}
impl WorkerImpl {
    fn work(&self) -> Result<()> {
        use crate::galery::{Gallery, ImmichGallery};
        if !set_current_thread_priority(ThreadPriority::Min).is_ok() {
            error!("Cannot change worker thread priority to minimal");
        }
        let mut immich = ImmichGallery::new(&self.config.source);
        loop {
            let mut img_with_details = immich.get_next_image();
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
