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
pub struct BufferObject<Type> {
    object: glow::NativeBuffer,
    target: BufferTarget,
    usage: BufferUsage,
    gl: GlContext,
    _data_type: PhantomData<Type>,
}

pub struct BindGuard<'a, T> {
    buffer: &'a BufferObject<T>,
}

impl<'a, T> Drop for BindGuard<'a, T> {
    fn drop(&mut self) {
        self.buffer.unbind();
    }
}

impl<Type: NoUninit> BufferObject<Type> {
    pub fn write(&self, data: &[Type]) {
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
            self.gl.buffer_data_u8_slice(
                self.target.get(),
                bytemuck::cast_slice(data),
                self.usage.get(),
            );
        }
    }
}

impl<Type> BufferObject<Type> {
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
    pub fn new_vertex_buffer(gl: GlContext, usage: BufferUsage) -> Self {
        BufferObject::new(gl, BufferTarget::ArrayBuffer, usage)
    }
    pub fn bind(&self) {
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
        }
    }
    pub fn unbind(&self) {
        unsafe {
            self.gl.bind_buffer(self.target.get(), None);
        }
    }
    pub fn bind_guard(&self) -> BindGuard<Type> {
        self.bind();
        BindGuard { buffer: self }
    }
}

impl BufferObject<u32> {
    pub fn new_index_buffer(gl: GlContext, usage: BufferUsage) -> Self {
        BufferObject::new(gl, BufferTarget::ElementArrayBuffer, usage)
    }
}

impl<T> Drop for BufferObject<T> {
    fn drop(&mut self) {
        unsafe { self.gl.delete_buffer(self.object) };
    }
}

pub type ElementBufferObject = BufferObject<u32>;
