mod http;
mod mqtt;

use anyhow::Result;
use tokio::{sync::watch, try_join};

use self::{http::HttpInterface, mqtt::MqttInterface};
use crate::configuration::Settings;

pub struct InterfaceManager {}

pub trait Interface {
    async fn start(&self, settings: watch::Sender<Settings>) -> Result<()>;
}

impl InterfaceManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&self, settings: watch::Sender<Settings>) -> Result<()> {
        let interface = HttpInterface;
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            runtime
                .block_on(async move {
                    let http = interface.start(settings.clone());
                    let mqtt = MqttInterface::new();
                    let mqtt = mqtt.start(settings.clone());
                    try_join!(http, mqtt)
                })
                .unwrap();
        });
        Ok(())
    }
}
