use anyhow::{Context, Result};
use application::Application;
use config::Config;
use log::debug;

use self::configuration::Conf;

mod application;
mod configuration;
mod gallery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() -> Result<()> {
    let config_path = std::env::var("CONFIG_PATH").unwrap_or("config".to_string());

    env_logger::init();
    let settings = Config::builder()
        .add_source(::config::File::with_name(&config_path))
        .build()
        .context("Cannot parse configuration")?;
    let config: Conf = settings
        .try_deserialize()
        .context("Cannot deserialize configuration")?;
    debug!("Configuration: {config:#?}");
    support::start::<Application>(config)?;
    Ok(())
}
