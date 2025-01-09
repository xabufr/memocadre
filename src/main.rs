#[macro_use]
extern crate glium;

mod galery;
mod graphics;
mod render;
mod support;

fn main() {
    env_logger::init();
    render::start();
}
