use anyhow::{Context, Error, Result};
use glow::{HasContext, NativeProgram};
use std::collections::HashMap;
use vek::{Extent2, Mat4, Vec2};

use super::GlContext;

type UniformLocation = glow::NativeUniformLocation;

pub struct ProgramGuard<'a> {
    program: &'a Program,
}

pub struct Program {
    program: NativeProgram,
    gl: GlContext,
    uniforms: HashMap<String, UniformLocation>,
}

impl Drop for ProgramGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.program.gl.use_program(None);
        }
    }
}

pub enum UniformValue {
    Float(f32),
    SignedInt(i32),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
    Vec4(f32, f32, f32, f32),
    Mat4([f32; 16]),
}

pub trait AsUniformValue {
    fn as_uniform_value(self) -> UniformValue;
}
impl AsUniformValue for UniformValue {
    fn as_uniform_value(self) -> UniformValue {
        self
    }
}

impl AsUniformValue for f32 {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Float(self)
    }
}
impl AsUniformValue for i32 {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::SignedInt(self)
    }
}
impl AsUniformValue for (f32, f32) {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Vec2(self.0, self.1)
    }
}
impl AsUniformValue for Extent2<f32> {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Vec2(self.w, self.h)
    }
}
impl AsUniformValue for Vec2<f32> {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Vec2(self.x, self.y)
    }
}
impl AsUniformValue for (f32, f32, f32) {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Vec3(self.0, self.1, self.2)
    }
}
impl AsUniformValue for (f32, f32, f32, f32) {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Vec4(self.0, self.1, self.2, self.3)
    }
}
impl AsUniformValue for Mat4<f32> {
    fn as_uniform_value(self) -> UniformValue {
        UniformValue::Mat4(self.into_col_array())
    }
}

impl<'a> ProgramGuard<'a> {
    pub fn set_uniform(&self, name: &str, value: impl AsUniformValue) {
        let location = self.program.uniforms.get(name).unwrap();
        let location = Some(location);
        let value = value.as_uniform_value();
        let gl = &self.program.gl;
        unsafe {
            match value {
                UniformValue::Float(f) => gl.uniform_1_f32(location, f),
                UniformValue::SignedInt(i) => gl.uniform_1_i32(location, i),
                UniformValue::Vec2(x, y) => gl.uniform_2_f32(location, x, y),
                UniformValue::Vec3(x, y, z) => gl.uniform_3_f32(location, x, y, z),
                UniformValue::Vec4(x, y, z, w) => gl.uniform_4_f32(location, x, y, z, w),
                UniformValue::Mat4(v) => gl.uniform_matrix_4_f32_slice(location, false, &v),
            }
        }
    }
}

impl Program {
    pub fn new(gl: GlContext, vertex: &str, fragment: &str) -> Result<Self> {
        let (program, uniforms) = unsafe {
            let vertex = Self::compile_shader(&gl, glow::VERTEX_SHADER, vertex)
                .context("Cannot compile vertex shader")?;
            let fragment = Self::compile_shader(&gl, glow::FRAGMENT_SHADER, fragment)
                .context("Cannot compile fragment shader")?;
            let program = Self::link_program(&gl, &[vertex, fragment])
                .context("Cannot link shader program")?;
            gl.delete_shader(vertex);
            gl.delete_shader(fragment);
            let uniforms = gl.get_program_parameter_i32(program, glow::ACTIVE_UNIFORMS);
            let uniforms = (0..uniforms)
                .map(|l| {
                    let info = gl
                        .get_active_uniform(program, l as u32)
                        .with_context(|| format!("Cannot get uniform #{l}"))?;
                    Ok((info.name.to_owned(), glow::NativeUniformLocation(l as _)))
                })
                .collect::<Result<HashMap<String, UniformLocation>>>()
                .context("While creating uniforms cache")?;
            (program, uniforms)
        };
        Ok(Self {
            program,
            uniforms,
            gl,
        })
    }
    pub fn get(&self) -> NativeProgram {
        return self.program;
    }
    pub fn bind(&self) -> ProgramGuard {
        unsafe {
            self.gl.use_program(Some(self.program));
        }
        ProgramGuard { program: self }
    }

    pub fn get_attrib_location(&self, name: &str) -> u32 {
        return unsafe { self.gl.get_attrib_location(self.program, name).unwrap() };
    }

    pub fn get_uniform_location(&self, name: &str) -> glow::NativeUniformLocation {
        return unsafe { self.gl.get_uniform_location(self.program, name).unwrap() };
    }

    unsafe fn compile_shader(
        gl: &glow::Context,
        shader_type: u32,
        source: &str,
    ) -> Result<glow::Shader> {
        unsafe {
            let shader = gl.create_shader(shader_type).map_err(Error::msg)?;

            gl.shader_source(shader, source);

            gl.compile_shader(shader);

            if gl.get_shader_compile_status(shader) {
                Ok(shader)
            } else {
                Err(Error::msg(gl.get_shader_info_log(shader)))
            }
        }
    }

    unsafe fn link_program<'a, T: IntoIterator<Item = &'a glow::Shader>>(
        gl: &glow::Context,
        shaders: T,
    ) -> Result<glow::Program> {
        unsafe {
            let program = gl.create_program().map_err(Error::msg)?;

            for shader in shaders {
                gl.attach_shader(program, *shader);
            }

            gl.link_program(program);

            if gl.get_program_link_status(program) {
                Ok(program)
            } else {
                Err(Error::msg(gl.get_program_info_log(program)))
            }
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
        }
    }
}
