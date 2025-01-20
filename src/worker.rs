use std::sync::{
    atomic::AtomicU32,
    mpsc::{Receiver, SyncSender},
    Arc, RwLock,
};

use glam::UVec2;
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use log::error;
use thread_priority::{set_current_thread_priority, ThreadPriority};

type Message = DynamicImage;
pub struct Worker {
    worker_impl: Arc<WorkerImpl>,
    recv: Receiver<Message>,
    send: SyncSender<Message>,
}

struct WorkerImpl {
    send: SyncSender<Message>,
    ideal_max_size: RwLock<UVec2>,
}

impl Worker {
    pub fn new(ideal_max_size: UVec2) -> Self {
        let (send, recv) = std::sync::mpsc::sync_channel(1);
        let worker_impl = Arc::new({
            let send = send.clone();
            WorkerImpl {
                send,
                ideal_max_size: RwLock::new(ideal_max_size),
            }
        });
        Worker {
            worker_impl,
            send,
            recv,
        }
    }

    pub fn set_ideal_max_size(&self, size: UVec2) {
        let mut w = self.worker_impl.ideal_max_size.write().unwrap();
        *w = size;
    }

    pub fn start(&self) {
        let worker_impl = self.worker_impl.clone();
        std::thread::spawn(move || {
            worker_impl.work();
        });
    }

    pub fn recv(&self) -> &Receiver<Message> {
        &self.recv
    }
}
impl WorkerImpl {
    fn work(&self) {
        use crate::galery::{Galery, ImmichGalery};
        if !set_current_thread_priority(ThreadPriority::Min).is_ok() {
            error!("Cannot change worker thread priority to minimal");
        }
        let mut immich = ImmichGalery::new(
            "***REMOVED***",
            "***REMOVED***",
        );
        loop {
            let img = immich.get_next_image();
            // sleep(Duration::from_secs(3600));
            let img = self.resize_image_if_necessay(img);
            self.send.send(img).unwrap();
        }
    }
    fn resize_image_if_necessay(&self, image: DynamicImage) -> DynamicImage {
        let image_dims: UVec2 = image.dimensions().into();
        let ideal_size = {
            let r = self.ideal_max_size.read().unwrap();
            r.clone()
        };
        let should_resize = image_dims.cmpgt(ideal_size).any();
        return if should_resize {
            image.resize(ideal_size.x, ideal_size.y, FilterType::Lanczos3)
        } else {
            image
        };
    }
}
// fn work() {
//     use crate::galery::{Galery, ImmichGalery};
//     if !set_current_thread_priority(ThreadPriority::Min).is_ok() {
//         error!("Cannot change worker thread priority to minimal");
//     }
//     let mut immich = ImmichGalery::new(
//         "***REMOVED***",
//         "***REMOVED***",
//     );
//     loop {
//         let img = immich.get_next_image();
//         // sleep(Duration::from_secs(3600));
//         send.send(img).unwrap();
//     }
// }
