pub use blur::ImageBlurr;
pub use image_display::ImageDisplay;

mod blur;
mod image_display;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex2dUv {
    pos: [f32; 2],
    uv: [f32; 2],
}
implement_vertex!(Vertex2dUv, pos, uv);
