use std::{marker::PhantomData, rc::Rc};

use anyhow::{bail, Error, Result};
use bytemuck::NoUninit;

use super::GlContext;

#[derive(Copy, Clone, Debug)]
pub enum BufferTarget {
    ArrayBuffer,
    ElementArrayBuffer,
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum BufferUsage {
    Static,
    Stream,
    Dynamic,
}

impl BufferTarget {
    fn to_gl(self) -> u32 {
        match self {
            BufferTarget::ArrayBuffer => glow::ARRAY_BUFFER,
            BufferTarget::ElementArrayBuffer => glow::ELEMENT_ARRAY_BUFFER,
        }
    }
}

impl BufferUsage {
    fn to_gl(self) -> u32 {
        match self {
            BufferUsage::Static => glow::STATIC_DRAW,
            BufferUsage::Stream => glow::STREAM_DRAW,
            BufferUsage::Dynamic => glow::DYNAMIC_DRAW,
        }
    }
}
pub struct BufferObject<Type> {
    object: glow::NativeBuffer,
    target: BufferTarget,
    usage: BufferUsage,
    gl: Rc<GlContext>,
    /// Size of the buffer in elements
    size: usize,
    _data_type: PhantomData<Type>,
}

impl<Type: NoUninit> BufferObject<Type> {
    pub fn write(&mut self, data: &[Type]) {
        self.size = data.len();
        unsafe {
            self.gl.bind_buffer(self.target.to_gl(), Some(self.object));
            self.gl.buffer_data_u8_slice(
                self.target.to_gl(),
                bytemuck::cast_slice(data),
                self.usage.to_gl(),
            );
        }
    }

    pub fn write_sub(&self, offset: usize, data: &[Type]) -> Result<()> {
        if offset + data.len() > self.size {
            bail!("BufferObject overflow");
        }
        let offset = offset * std::mem::size_of::<Type>();
        unsafe {
            self.gl.bind_buffer(self.target.to_gl(), Some(self.object));
            self.gl.buffer_sub_data_u8_slice(
                self.target.to_gl(),
                offset as _,
                bytemuck::cast_slice(data),
            );
        }
        Ok(())
    }
}

impl<Type> BufferObject<Type> {
    fn new(gl: Rc<GlContext>, target: BufferTarget, usage: BufferUsage) -> Result<Self> {
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

    pub fn new_vertex_buffer(gl: Rc<GlContext>, usage: BufferUsage) -> Result<Self> {
        BufferObject::new(gl, BufferTarget::ArrayBuffer, usage)
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.bind_buffer(self.target.to_gl(), Some(self.object));
        }
    }

    pub fn unbind(&self) {
        unsafe {
            self.gl.bind_buffer(self.target.to_gl(), None);
        }
    }

    /// Size of the buffer in elements
    pub fn size(&self) -> usize {
        self.size
    }
}

impl BufferObject<u32> {
    pub fn new_index_buffer(gl: Rc<GlContext>, usage: BufferUsage) -> Result<Self> {
        BufferObject::new(gl, BufferTarget::ElementArrayBuffer, usage)
    }
}

impl<T> Drop for BufferObject<T> {
    fn drop(&mut self) {
        unsafe { self.gl.delete_buffer(self.object) };
    }
}

pub type ElementBufferObject = BufferObject<u32>;
