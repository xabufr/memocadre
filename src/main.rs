use anyhow::{Context, Result};
use application::Application;
use config::Config;
use log::debug;
use schematic::{
    schema::{JsoncTemplateRenderer, SchemaGenerator, TemplateOptions, YamlTemplateRenderer},
    ConfigLoader,
};

use self::configuration::Conf;

mod application;
mod configuration;
mod gallery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() -> Result<()> {
    let config_path = std::env::var("CONFIG_PATH").unwrap_or("config.yaml".to_string());

    env_logger::init();
    let mut generator = SchemaGenerator::default();
    let options = TemplateOptions {
        expand_fields: vec![
            "sources".into(),
            "sources.instances".into(),
            "sources.specs".into(),
        ],
        ..Default::default()
    };
    let renderer = JsoncTemplateRenderer::new(options);

    generator.add::<Conf>();
    generator.generate("test.json", renderer).unwrap();
    let config = ConfigLoader::<Conf>::new().file(config_path)?.load()?;

    debug!("Configuration: {:#?}", config.config);
    support::start::<Application>(config.config)?;
    Ok(())
}
