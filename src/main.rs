use confique::Config;

mod application;
mod config;
mod galery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() {
    let config = config::Conf::builder().file("config.toml").load().unwrap();
    env_logger::init();
    application::start(config);
}
