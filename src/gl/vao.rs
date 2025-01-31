use anyhow::{Error, Result};
use bytemuck::NoUninit;
use glow::HasContext;

use super::{
    buffer_object::{BufferObject, ElementBufferObject},
    GlContext,
};

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct BufferInfo {
    pub location: u32, //
    pub vector_size: i32,
    pub data_type: u32, //GL_FLOAT,GL_UNSIGNED_BYTE
    pub normalized: bool,
    pub stride: i32,
    pub offset: i32,
}

// ----------------------------------------------------------------------------

pub struct VaoBindGuard<'a, V> {
    array_object: &'a VertexArrayObject<V>,
}
impl<V> Drop for VaoBindGuard<'_, V> {
    fn drop(&mut self) {
        if self.array_object.vao.is_some() {
            unsafe { self.array_object.gl.bind_vertex_array(None) };
        } else {
            self.array_object.vertex_buffer.unbind();
            self.array_object.element_buffer.unbind();
        }
    }
}

/// Wrapper around either Emulated VAO or GL's VAO.
pub struct VertexArrayObject<V> {
    pub vertex_buffer: BufferObject<V>,
    pub element_buffer: ElementBufferObject,
    // If `None`, we emulate VAO:s.
    vao: Option<glow::VertexArray>,
    buffer_infos: Vec<BufferInfo>,
    gl: GlContext,
}

impl<V: NoUninit> VertexArrayObject<V> {
    pub fn new(
        gl: GlContext,
        vbo: BufferObject<V>,
        ebo: ElementBufferObject,
        buffer_infos: Vec<BufferInfo>,
    ) -> Result<Self> {
        let vao = if supports_vao(&gl) {
            unsafe {
                let vao = gl.create_vertex_array().map_err(Error::msg)?;

                // Store state in the VAO:
                gl.bind_vertex_array(Some(vao));

                vbo.bind();
                ebo.bind();

                Self::bind_attributes(&gl, &buffer_infos);

                gl.bind_vertex_array(None);

                Some(vao)
            }
        } else {
            log::debug!("VAO not supported");
            None
        };

        Ok(Self {
            vao,
            vertex_buffer: vbo,
            element_buffer: ebo,
            buffer_infos,
            gl,
        })
    }
}
impl<V> VertexArrayObject<V> {
    fn bind_attributes(gl: &GlContext, buffer_infos: &[BufferInfo]) {
        unsafe {
            for attribute in buffer_infos {
                gl.vertex_attrib_pointer_f32(
                    attribute.location,
                    attribute.vector_size,
                    attribute.data_type,
                    attribute.normalized,
                    attribute.stride,
                    attribute.offset,
                );
                gl.enable_vertex_attrib_array(attribute.location);
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            if let Some(vao) = self.vao {
                self.gl.bind_vertex_array(Some(vao));
            } else {
                self.vertex_buffer.bind();
                self.element_buffer.bind();

                Self::bind_attributes(&self.gl, &self.buffer_infos);
            }
        }
    }

    pub fn bind_guard(&self) -> VaoBindGuard<V> {
        self.bind();
        VaoBindGuard { array_object: self }
    }
}

impl<V> Drop for VertexArrayObject<V> {
    fn drop(&mut self) {
        if let Some(vao) = self.vao {
            unsafe {
                self.gl.delete_vertex_array(vao);
            }
        }
    }
}
// ----------------------------------------------------------------------------

fn supports_vao(gl: &glow::Context) -> bool {
    const WEBGL_PREFIX: &str = "WebGL ";
    const OPENGL_ES_PREFIX: &str = "OpenGL ES ";

    let version_string = unsafe { gl.get_parameter_string(glow::VERSION) };
    log::debug!("GL version: {:?}.", version_string);

    // Examples:
    // * "WebGL 2.0 (OpenGL ES 3.0 Chromium)"
    // * "WebGL 2.0"

    if let Some(pos) = version_string.rfind(WEBGL_PREFIX) {
        let version_str = &version_string[pos + WEBGL_PREFIX.len()..];
        if version_str.contains("1.0") {
            // need to test OES_vertex_array_object .
            let supported_extensions = gl.supported_extensions();
            log::debug!("Supported OpenGL extensions: {:?}", supported_extensions);
            supported_extensions.contains("OES_vertex_array_object")
                || supported_extensions.contains("GL_OES_vertex_array_object")
        } else {
            true
        }
    } else if version_string.contains(OPENGL_ES_PREFIX) {
        // glow targets es2.0+ so we don't concern about OpenGL ES-CM,OpenGL ES-CL
        if version_string.contains("2.0") {
            // need to test OES_vertex_array_object .
            let supported_extensions = gl.supported_extensions();
            log::debug!("Supported OpenGL extensions: {:?}", supported_extensions);
            supported_extensions.contains("OES_vertex_array_object")
                || supported_extensions.contains("GL_OES_vertex_array_object")
        } else {
            true
        }
    } else {
        // from OpenGL 3 vao into core
        if version_string.starts_with('2') {
            // I found APPLE_vertex_array_object , GL_ATI_vertex_array_object ,ARB_vertex_array_object
            // but APPLE's and ATI's very old extension.
            let supported_extensions = gl.supported_extensions();
            log::debug!("Supported OpenGL extensions: {:?}", supported_extensions);
            supported_extensions.contains("ARB_vertex_array_object")
                || supported_extensions.contains("GL_ARB_vertex_array_object")
        } else {
            true
        }
    }
}
