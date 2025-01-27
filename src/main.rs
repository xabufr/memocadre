use anyhow::{Context, Result};
use config::Config;
use configuration::Conf;
use log::debug;

mod application;
mod configuration;
mod galery;
mod gl;
mod graphics;
mod support;
mod worker;

fn main() -> Result<()> {
    let settings = Config::builder()
        .add_source(::config::File::with_name("config"))
        .build()
        .context("Cannot parse configuration")?;
    let config: Conf = settings
        .try_deserialize()
        .context("Cannot deserialize configuration")?;
    debug!("Configuration: {config:#?}");
    env_logger::init();
    application::start(config)?;
    Ok(())
}
