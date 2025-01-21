mod application;
mod galery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() {
    env_logger::init();
    application::start();
}
