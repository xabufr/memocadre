use config::Config;
use configuration::Conf;

mod application;
mod configuration;
mod galery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() {
    let settings = Config::builder()
        .add_source(::config::File::with_name("config"))
        .build()
        .unwrap();
    let config: Conf = settings.try_deserialize().unwrap();
    env_logger::init();
    application::start(config);
}
