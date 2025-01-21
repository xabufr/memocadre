use glow::HasContext;

use super::{GlContext, Texture, Viewport};

pub struct FramebufferGuard<'a> {
    previous_viewport: Viewport,
    framebuffer: &'a FramebufferObject,
}

impl<'a> Drop for FramebufferGuard<'a> {
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
    gl: GlContext,
}

impl FramebufferObject {
    pub fn with_texture(gl: GlContext, texture: Texture) -> Self {
        unsafe {
            let fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture.get()),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            return Self {
                framebuffer: fbo,
                texture: Some(texture),
                gl,
            };
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
        let texture = self.texture.as_ref().unwrap();
        self.gl
            .set_viewport((0, 0, texture.size().x as i32, texture.size().y as i32));
        self.bind();
        FramebufferGuard {
            previous_viewport,
            framebuffer: self,
        }
    }
    pub fn into_texture(mut self) -> Texture {
        self.texture.take().unwrap()
    }
    pub fn get_texture(&self) -> &Texture {
        self.texture.as_ref().unwrap()
    }
}

impl Drop for FramebufferObject {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_framebuffer(self.framebuffer);
        }
    }
}
