use anyhow::{Context, Result};
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
    env_logger::init();
    let settings = Config::builder()
        .add_source(::config::File::with_name("config"))
        .build()
        .context("Cannot parse configuration")?;
    let config: Conf = settings
        .try_deserialize()
        .context("Cannot deserialize configuration")?;
    debug!("Configuration: {config:#?}");
    application::start(config)?;
    Ok(())
}
