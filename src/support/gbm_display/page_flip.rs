use anyhow::Result;
use drm::control::Device as ControlDevice;

use super::drm_device::DrmDevice;

type FbHandle = drm::control::framebuffer::Handle;

pub struct PageFlipper<'a> {
    device: &'a DrmDevice,
    surface: &'a gbm::Surface<()>,
    bpp: u32,
}

impl<'a> PageFlipper<'a> {
    pub fn new(device: &'a DrmDevice, surface: &'a gbm::Surface<()>) -> Self {
        Self {
            device,
            surface,
            bpp: 0,
        }
    }

    pub fn initial_flip(&mut self) -> Result<(gbm::BufferObject<()>, FbHandle)> {
        let bo = unsafe { self.surface.lock_front_buffer()? };
        self.bpp = bo.bpp();

        let fb = self.device.add_framebuffer(&bo, self.bpp, self.bpp)?;
        self.device.init_crtc(fb)?;

        Ok((bo, fb))
    }

    pub fn flip(&self, bo: &mut gbm::BufferObject<()>, fb: &mut FbHandle) -> Result<()> {
        let next_bo = unsafe { self.surface.lock_front_buffer()? };
        let next_fb = self.device.add_framebuffer(&next_bo, self.bpp, self.bpp)?;

        self.device.flip_and_wait(next_fb)?;

        drop(std::mem::replace(bo, next_bo));
        self.device
            .destroy_framebuffer(std::mem::replace(fb, next_fb))?;
        Ok(())
    }
}
