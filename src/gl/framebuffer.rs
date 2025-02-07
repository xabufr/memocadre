use std::rc::Rc;

use anyhow::{Error, Result};
use vek::Rect;

use super::{texture::Texture, GlContext};

pub struct FramebufferGuard<'a> {
    previous_viewport: Rect<i32, i32>,
    framebuffer: &'a FramebufferObject,
}

impl Drop for FramebufferGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.framebuffer
                .gl
                .bind_framebuffer(glow::FRAMEBUFFER, None);
            self.framebuffer.gl.set_viewport(self.previous_viewport);
        }
    }
}

pub struct FramebufferObject {
    framebuffer: glow::NativeFramebuffer,
    texture: Option<Texture>,
    gl: Rc<GlContext>,
}

impl FramebufferObject {
    pub fn with_texture(gl: Rc<GlContext>, texture: Texture) -> Result<Self> {
        unsafe {
            let fbo = gl.create_framebuffer().map_err(Error::msg)?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.get()),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            Ok(Self {
                framebuffer: fbo,
                texture: Some(texture),
                gl,
            })
        }
    }

    fn bind(&self) {
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
        }
    }

    pub fn bind_guard(&self) -> FramebufferGuard {
        let previous_viewport = self.gl.current_viewport();
        let texture = self.texture.as_ref().expect("Texture should be present");
        self.gl.set_viewport(Rect::new(
            0,
            0,
            texture.size().w as i32,
            texture.size().h as i32,
        ));
        self.bind();
        FramebufferGuard {
            previous_viewport,
            framebuffer: self,
        }
    }

    pub fn into_texture(mut self) -> Texture {
        self.texture.take().expect("Texture should be present")
    }

    pub fn get_texture(&self) -> &Texture {
        self.texture.as_ref().expect("Texture should be present")
    }
}

impl Drop for FramebufferObject {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.framebuffer);
        }
    }
}
