mod application;
mod configuration;
mod gallery;
mod gl;
mod graphics;
mod support;
mod worker;

use anyhow::Result;

use self::application::Application;

fn main() -> Result<()> {
    env_logger::init();
    support::start::<Application>()?;
    Ok(())
}
