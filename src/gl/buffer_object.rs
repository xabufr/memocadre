use anyhow::{bail, Context, Error, Result};
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
#[allow(dead_code)]
pub enum BufferUsage {
    StaticDraw,
    StreamDraw,
    DynamicDraw,
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
            BufferUsage::StreamDraw => glow::STREAM_DRAW,
            BufferUsage::DynamicDraw => glow::DYNAMIC_DRAW,
        }
    }
}
pub struct BufferObject<Type> {
    object: glow::NativeBuffer,
    target: BufferTarget,
    usage: BufferUsage,
    gl: GlContext,
    /// Size of the buffer in elements
    size: usize,
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
    pub fn write(&mut self, data: &[Type]) {
        self.size = data.len();
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
            self.gl.buffer_data_u8_slice(
                self.target.get(),
                bytemuck::cast_slice(data),
                self.usage.get(),
            );
        }
    }
    pub fn write_sub(&self, offset: usize, data: &[Type]) -> Result<()> {
        if offset + data.len() > self.size {
            bail!("BufferObject overflow");
        }
        let offset = offset * std::mem::size_of::<Type>();
        unsafe {
            self.gl.bind_buffer(self.target.get(), Some(self.object));
            self.gl.buffer_sub_data_u8_slice(
                self.target.get(),
                offset as _,
                bytemuck::cast_slice(data),
            );
        }
        Ok(())
    }
}

impl<Type> BufferObject<Type> {
    fn new(gl: GlContext, target: BufferTarget, usage: BufferUsage) -> Result<Self> {
        let object = unsafe { gl.create_buffer().map_err(Error::msg)? };
        Ok(BufferObject {
            object,
            target,
            usage,
            gl,
            size: 0,
            _data_type: PhantomData,
        })
    }
    pub fn new_vertex_buffer(gl: GlContext, usage: BufferUsage) -> Result<Self> {
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
    /// Size of the buffer in elements
    pub fn size(&self) -> usize {
        self.size
    }
}

impl BufferObject<u32> {
    pub fn new_index_buffer(gl: GlContext, usage: BufferUsage) -> Result<Self> {
        BufferObject::new(gl, BufferTarget::ElementArrayBuffer, usage)
    }
}

impl<T> Drop for BufferObject<T> {
    fn drop(&mut self) {
        unsafe { self.gl.delete_buffer(self.object) };
    }
}

pub type ElementBufferObject = BufferObject<u32>;
