#[macro_use]
extern crate glium;

mod application;
mod galery;
mod graphics;
mod support;
mod worker;

fn main() {
    env_logger::init();
    application::start();
}
