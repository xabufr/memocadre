use glow::{HasContext, NativeProgram};

pub struct Program {
    program: NativeProgram,
}

impl Program {
    pub fn new(gl: &glow::Context, vertex: &str, fragment: &str) -> Self {
        let program = unsafe {
            let vertex = compile_shader(gl, glow::VERTEX_SHADER, vertex).unwrap();
            let fragment = compile_shader(gl, glow::FRAGMENT_SHADER, fragment).unwrap();
            let program = link_program(gl, &[vertex, fragment]).unwrap();
            // gl.delete_shader(vertex);
            // gl.delete_shader(fragment);
            program
        };
        Self { program }
    }
    pub fn get(&self) -> NativeProgram {
        return self.program;
    }

    pub fn get_attrib_location(&self, gl: &glow::Context, name: &str) -> Option<u32> {
        return unsafe { gl.get_attrib_location(self.program, name) };
    }

    pub fn get_uniform_location(
        &self,
        gl: &glow::Context,
        name: &str,
    ) -> Option<glow::NativeUniformLocation> {
        return unsafe { gl.get_uniform_location(self.program, name) };
    }
}

pub unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl.create_shader(shader_type)?;

        gl.shader_source(shader, source);

        gl.compile_shader(shader);

        if gl.get_shader_compile_status(shader) {
            Ok(shader)
        } else {
            Err(gl.get_shader_info_log(shader))
        }
    }
}

pub unsafe fn link_program<'a, T: IntoIterator<Item = &'a glow::Shader>>(
    gl: &glow::Context,
    shaders: T,
) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;

        for shader in shaders {
            gl.attach_shader(program, *shader);
        }

        gl.link_program(program);

        if gl.get_program_link_status(program) {
            Ok(program)
        } else {
            Err(gl.get_program_info_log(program))
        }
    }
}
