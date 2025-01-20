use std::marker::PhantomData;

use super::GlContext;
use bytemuck::NoUninit;
use glow::HasContext;

#[derive(Copy, Clone, Debug)]
pub enum BufferTarget {
    ArrayBuffer,
    ElementArrayBuffer,
}

#[derive(Copy, Clone, Debug)]
pub enum BufferUsage {
    StaticDraw,
}

impl BufferTarget {
    fn get(&self) -> u32 {
        match self {
            BufferTarget::ArrayBuffer => glow::ARRAY_BUFFER,
            BufferTarget::ElementArrayBuffer => glow::ELEMENT_ARRAY_BUFFER,
        }
    }
}

impl BufferUsage {
    fn get(&self) -> u32 {
        match self {
            BufferUsage::StaticDraw => glow::STATIC_DRAW,
        }
    }
}
struct BufferObject<Type> {
    object: glow::NativeBuffer,
    target: BufferTarget,
    usage: BufferUsage,
    gl: GlContext,
    _data_type: PhantomData<Type>,
}

impl<Type: NoUninit> BufferObject<Type> {
    fn new(gl: GlContext, target: BufferTarget, usage: BufferUsage) -> Self {
        let object = unsafe { gl.create_buffer().unwrap() };
        BufferObject {
            object,
            target,
            usage,
            gl,
            _data_type: PhantomData,
        }
    }
    fn write(&self, data: &[Type]) {
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
            self.gl.buffer_data_u8_slice(
                self.target.get(),
                bytemuck::cast_slice(data),
                self.usage.get(),
            );
        }
    }
    fn bind(&self) {
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
        }
    }
}

impl<T> Drop for BufferObject<T> {
    fn drop(&mut self) {
        unsafe { self.gl.delete_buffer(self.object) };
    }
}

pub struct ElementBufferObject(BufferObject<u32>);

impl ElementBufferObject {
    pub fn new(gl: GlContext, usage: BufferUsage) -> Self {
        Self(BufferObject::new(
            gl,
            BufferTarget::ElementArrayBuffer,
            usage,
        ))
    }
    pub fn write(&self, gl: &glow::Context, data: &[u32]) {
        self.0.write(data);
    }
    pub fn bind(&self, gl: &glow::Context) {
        self.0.bind();
    }
}
