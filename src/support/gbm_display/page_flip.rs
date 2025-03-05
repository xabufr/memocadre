use anyhow::Result;
use drm::control::Device as ControlDevice;

use super::drm_device::DrmDevice;

type FbHandle = drm::control::framebuffer::Handle;

pub struct PageFlipper<'a> {
    device: &'a DrmDevice,
    surface: &'a gbm::Surface<()>,
    bo: gbm::BufferObject<()>,
    fb: FbWrapper<'a>,
    bpp: u32,
}

struct FbWrapper<'a> {
    handle: FbHandle,
    device: &'a DrmDevice,
}

impl Drop for FbWrapper<'_> {
    fn drop(&mut self) {
        if let Err(err) = self.device.destroy_framebuffer(self.handle) {
            log::error!("Failed to destroy framebuffer: {}", err);
        }
    }
}

impl<'a> PageFlipper<'a> {
    pub fn init(device: &'a DrmDevice, surface: &'a gbm::Surface<()>) -> Result<Self> {
        let bo = unsafe { surface.lock_front_buffer()? };
        let bpp = bo.bpp();

        let fb = FbWrapper {
            handle: device.add_framebuffer(&bo, bpp, bpp)?,
            device,
        };
        device.init_crtc(fb.handle)?;

        Ok(Self {
            device,
            surface,
            fb,
            bo,
            bpp,
        })
    }

    pub fn flip(&mut self) -> Result<()> {
        let next_bo = unsafe { self.surface.lock_front_buffer()? };
        let next_fb = FbWrapper {
            handle: self.device.add_framebuffer(&next_bo, self.bpp, self.bpp)?,
            device: self.device,
        };

        self.device.flip_and_wait(next_fb.handle)?;

        drop(std::mem::replace(&mut self.bo, next_bo));
        drop(std::mem::replace(&mut self.fb, next_fb));
        Ok(())
    }
}
