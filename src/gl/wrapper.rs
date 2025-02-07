use std::collections::HashSet;

use glow::{
    ActiveUniform, HasContext, NativeBuffer, NativeFramebuffer, NativeProgram, NativeShader,
    NativeTexture, NativeUniformLocation, NativeVertexArray, PixelUnpackData,
};

#[cfg_attr(test, faux::create)]
#[derive(Debug)]
pub struct GlowContext(glow::Context);

#[cfg_attr(test, faux::methods)]
impl From<glow::Context> for GlowContext {
    fn from(gl: glow::Context) -> Self {
        Self(gl)
    }
}

#[cfg_attr(test, faux::methods)]
impl GlowContext {
    #[inline(always)]
    pub unsafe fn viewport(&self, x: i32, y: i32, w: i32, h: i32) {
        self.0.viewport(x, y, w, h)
    }

    #[inline(always)]
    pub unsafe fn clear(&self, mask: u32) {
        self.0.clear(mask)
    }

    #[inline(always)]
    pub unsafe fn draw_elements(&self, mode: u32, count: i32, element_type: u32, offset: i32) {
        self.0.draw_elements(mode, count, element_type, offset)
    }

    #[inline(always)]
    pub unsafe fn enable(&self, parameter: u32) {
        self.0.enable(parameter)
    }

    #[inline(always)]
    pub unsafe fn disable(&self, parameter: u32) {
        self.0.disable(parameter)
    }

    #[inline(always)]
    pub unsafe fn blend_func_separate(
        &self,
        src_rgb: u32,
        dst_rgb: u32,
        src_alpha: u32,
        dst_alpha: u32,
    ) {
        self.0
            .blend_func_separate(src_rgb, dst_rgb, src_alpha, dst_alpha)
    }

    #[inline(always)]
    pub unsafe fn blend_equation_separate(&self, mode_rgb: u32, mode_alpha: u32) {
        self.0.blend_equation_separate(mode_rgb, mode_alpha)
    }

    #[inline(always)]
    pub unsafe fn get_parameter_i32(&self, parameter: u32) -> i32 {
        self.0.get_parameter_i32(parameter)
    }

    #[inline(always)]
    pub unsafe fn delete_vertex_array(&self, vertex_array: NativeVertexArray) {
        self.0.delete_vertex_array(vertex_array)
    }

    #[inline(always)]
    pub unsafe fn bind_vertex_array(&self, vertex_array: Option<NativeVertexArray>) {
        self.0.bind_vertex_array(vertex_array)
    }

    #[inline(always)]
    pub unsafe fn enable_vertex_attrib_array(&self, index: u32) {
        self.0.enable_vertex_attrib_array(index)
    }

    #[inline(always)]
    pub unsafe fn vertex_attrib_pointer_f32(
        &self,
        index: u32,
        size: i32,
        data_type: u32,
        normalized: bool,
        stride: i32,
        offset: i32,
    ) {
        self.0
            .vertex_attrib_pointer_f32(index, size, data_type, normalized, stride, offset)
    }

    #[inline(always)]
    pub unsafe fn delete_texture(&self, texture: NativeTexture) {
        self.0.delete_texture(texture)
    }

    #[inline(always)]
    pub unsafe fn bind_texture(&self, target: u32, texture: Option<NativeTexture>) {
        self.0.bind_texture(target, texture)
    }

    #[inline(always)]
    pub unsafe fn active_texture(&self, unit: u32) {
        self.0.active_texture(unit)
    }

    #[inline(always)]
    pub unsafe fn tex_sub_image_2d<'a>(
        &self,
        target: u32,
        level: i32,
        x_offset: i32,
        y_offset: i32,
        width: i32,
        height: i32,
        format: u32,
        ty: u32,
        pixels: PixelUnpackData<'a>,
    ) {
        self.0.tex_sub_image_2d(
            target, level, x_offset, y_offset, width, height, format, ty, pixels,
        )
    }

    #[inline(always)]
    pub unsafe fn tex_image_2d<'a>(
        &self,
        target: u32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: u32,
        ty: u32,
        pixels: PixelUnpackData<'a>,
    ) {
        self.0.tex_image_2d(
            target,
            level,
            internal_format,
            width,
            height,
            border,
            format,
            ty,
            pixels,
        )
    }

    #[inline(always)]
    pub unsafe fn create_vertex_array(&self) -> Result<NativeVertexArray, String> {
        self.0.create_vertex_array()
    }

    #[inline(always)]
    pub unsafe fn tex_parameter_i32(&self, target: u32, parameter: u32, value: i32) {
        self.0.tex_parameter_i32(target, parameter, value)
    }

    #[inline(always)]
    pub unsafe fn delete_program(&self, program: NativeProgram) {
        self.0.delete_program(program)
    }

    #[inline(always)]
    pub unsafe fn create_texture(&self) -> Result<NativeTexture, String> {
        self.0.create_texture()
    }

    #[inline(always)]
    pub unsafe fn uniform_matrix_4_f32_slice(
        &self,
        location: Option<&NativeUniformLocation>,
        transpose: bool,
        v: &[f32],
    ) {
        self.0.uniform_matrix_4_f32_slice(location, transpose, v)
    }

    #[inline(always)]
    pub unsafe fn uniform_4_f32(
        &self,
        location: Option<&NativeUniformLocation>,
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    ) {
        self.0.uniform_4_f32(location, x, y, z, w)
    }

    #[inline(always)]
    pub unsafe fn uniform_3_f32(
        &self,
        location: Option<&NativeUniformLocation>,
        x: f32,
        y: f32,
        z: f32,
    ) {
        self.0.uniform_3_f32(location, x, y, z)
    }

    #[inline(always)]
    pub unsafe fn uniform_2_f32(&self, location: Option<&NativeUniformLocation>, x: f32, y: f32) {
        self.0.uniform_2_f32(location, x, y)
    }

    #[inline(always)]
    pub unsafe fn uniform_1_f32(&self, location: Option<&NativeUniformLocation>, x: f32) {
        self.0.uniform_1_f32(location, x)
    }

    #[inline(always)]
    pub unsafe fn uniform_1_i32(&self, location: Option<&NativeUniformLocation>, x: i32) {
        self.0.uniform_1_i32(location, x)
    }

    #[inline(always)]
    pub unsafe fn get_attrib_location(&self, program: NativeProgram, name: &str) -> Option<u32> {
        self.0.get_attrib_location(program, name)
    }

    #[inline(always)]
    pub unsafe fn use_program(&self, program: Option<NativeProgram>) {
        self.0.use_program(program)
    }

    #[inline(always)]
    pub unsafe fn get_active_uniform(
        &self,
        program: NativeProgram,
        index: u32,
    ) -> Option<ActiveUniform> {
        self.0.get_active_uniform(program, index)
    }

    #[inline(always)]
    pub unsafe fn get_program_parameter_i32(&self, program: NativeProgram, parameter: u32) -> i32 {
        self.0.get_program_parameter_i32(program, parameter)
    }

    #[inline(always)]
    pub unsafe fn delete_shader(&self, shader: NativeShader) {
        self.0.delete_shader(shader)
    }

    #[inline(always)]
    pub unsafe fn delete_framebuffer(&self, framebuffer: NativeFramebuffer) {
        self.0.delete_framebuffer(framebuffer)
    }

    #[inline(always)]
    pub unsafe fn bind_framebuffer(&self, target: u32, framebuffer: Option<NativeFramebuffer>) {
        self.0.bind_framebuffer(target, framebuffer)
    }

    #[inline(always)]
    pub unsafe fn framebuffer_texture_2d(
        &self,
        target: u32,
        attachment: u32,
        texture_target: u32,
        texture: Option<NativeTexture>,
        level: i32,
    ) {
        self.0
            .framebuffer_texture_2d(target, attachment, texture_target, texture, level)
    }

    #[inline(always)]
    pub unsafe fn create_framebuffer(&self) -> Result<NativeFramebuffer, String> {
        self.0.create_framebuffer()
    }

    #[inline(always)]
    pub unsafe fn delete_buffer(&self, buffer: NativeBuffer) {
        self.0.delete_buffer(buffer)
    }

    #[inline(always)]
    pub unsafe fn bind_buffer(&self, target: u32, buffer: Option<NativeBuffer>) {
        self.0.bind_buffer(target, buffer)
    }

    #[inline(always)]
    pub unsafe fn create_buffer(&self) -> Result<NativeBuffer, String> {
        self.0.create_buffer()
    }

    #[inline(always)]
    pub unsafe fn buffer_sub_data_u8_slice(&self, target: u32, offset: i32, src_data: &[u8]) {
        self.0.buffer_sub_data_u8_slice(target, offset, src_data)
    }

    #[inline(always)]
    pub unsafe fn buffer_data_u8_slice(&self, target: u32, data: &[u8], usage: u32) {
        self.0.buffer_data_u8_slice(target, data, usage)
    }

    #[inline(always)]
    pub unsafe fn create_shader(&self, shader_type: u32) -> Result<NativeShader, String> {
        self.0.create_shader(shader_type)
    }

    #[inline(always)]
    pub unsafe fn shader_source(&self, shader: NativeShader, source: &str) {
        self.0.shader_source(shader, source)
    }

    #[inline(always)]
    pub unsafe fn compile_shader(&self, shader: NativeShader) {
        self.0.compile_shader(shader)
    }

    #[inline(always)]
    pub unsafe fn get_shader_compile_status(&self, shader: NativeShader) -> bool {
        self.0.get_shader_compile_status(shader)
    }

    #[inline(always)]
    pub unsafe fn get_shader_info_log(&self, shader: NativeShader) -> String {
        self.0.get_shader_info_log(shader)
    }

    #[inline(always)]
    pub unsafe fn create_program(&self) -> Result<NativeProgram, String> {
        self.0.create_program()
    }

    #[inline(always)]
    pub unsafe fn attach_shader(&self, program: NativeProgram, shader: NativeShader) {
        self.0.attach_shader(program, shader)
    }

    #[inline(always)]
    pub unsafe fn link_program(&self, program: NativeProgram) {
        self.0.link_program(program)
    }

    #[inline(always)]
    pub unsafe fn get_program_link_status(&self, program: NativeProgram) -> bool {
        self.0.get_program_link_status(program)
    }

    #[inline(always)]
    pub unsafe fn get_program_info_log(&self, program: NativeProgram) -> String {
        self.0.get_program_info_log(program)
    }

    #[inline(always)]
    pub fn supported_extensions(&self) -> &HashSet<String> {
        self.0.supported_extensions()
    }

    #[inline(always)]
    pub unsafe fn get_parameter_string(&self, parameter: u32) -> String {
        self.0.get_parameter_string(parameter)
    }

    #[inline(always)]
    pub unsafe fn pixel_store_i32(&self, parameter: u32, value: i32) {
        self.0.pixel_store_i32(parameter, value)
    }

    #[inline(always)]
    pub unsafe fn finish(&self) {
        self.0.finish()
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU32;

    use faux::when;
    use glow::{
        ActiveUniform, NativeBuffer, NativeProgram, NativeShader, NativeTexture, NativeVertexArray,
    };

    use super::GlowContext;

    pub fn mocked_gl() -> GlowContext {
        let mut gl = GlowContext::faux();
        when!(gl.delete_texture).then_return(());
        when!(gl.delete_buffer).then_return(());
        when!(gl.create_buffer).then_return(Ok(NativeBuffer(NonZeroU32::new(1).unwrap())));
        when!(gl.create_shader).then_return(Ok(NativeShader(NonZeroU32::new(1).unwrap())));
        when!(gl.shader_source).then_return(());
        when!(gl.compile_shader).then_return(());
        when!(gl.get_shader_compile_status).then_return(true);
        when!(gl.create_program).then_return(Ok(NativeProgram(NonZeroU32::new(1).unwrap())));
        when!(gl.attach_shader).then_return(());
        when!(gl.link_program).then_return(());
        when!(gl.get_program_link_status).then_return(true);
        when!(gl.delete_shader).then_return(());
        when!(gl.delete_program).then_return(());
        when!(gl.get_program_parameter_i32).then_return(8);
        when!(gl.get_attrib_location).then_return(Some(1));
        when!(gl.get_active_uniform).then(|(_, i)| {
            let n = match i {
                0 => "view",
                1 => "position",
                2 => "model",
                3 => "tex",
                4 => "uv_offset_center",
                5 => "uv_offset_size",
                6 => "tex_size",
                7 => "dir",
                _ => return None,
            };
            Some(ActiveUniform {
                name: n.to_string(),
                size: 1,
                utype: glow::FLOAT,
            })
        });
        when!(gl.bind_buffer).then_return(());
        when!(gl.bind_framebuffer).then_return(());
        when!(gl.bind_texture).then_return(());
        when!(gl.bind_vertex_array).then_return(());
        when!(gl.buffer_data_u8_slice).then_return(());
        when!(gl.buffer_sub_data_u8_slice).then_return(());
        when!(gl.framebuffer_texture_2d).then_return(());
        when!(gl.get_parameter_string).then_return("OpenGL ES 3.0".into());
        when!(gl.create_vertex_array)
            .then_return(Ok(NativeVertexArray(NonZeroU32::new(1).unwrap())));
        when!(gl.delete_vertex_array).then_return(());
        when!(gl.create_texture).then_return(Ok(NativeTexture(NonZeroU32::new(1).unwrap())));
        when!(gl.tex_image_2d).then_return(());
        when!(gl.tex_parameter_i32).then_return(());
        when!(gl.tex_sub_image_2d).then_return(());
        when!(gl.vertex_attrib_pointer_f32).then_return(());
        when!(gl.enable_vertex_attrib_array).then_return(());
        gl
    }
}

#[cfg(test)]
pub use test::mocked_gl;
