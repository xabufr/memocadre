mod http;
mod mqtt;

use std::{sync::mpsc, thread};

use anyhow::{Context, Result};
use tokio::{sync::watch, try_join};

use self::{http::HttpInterface, mqtt::MqttInterface};
use super::{ApplicationState, ControlCommand};
use crate::configuration::{AppConfig, HttpConfig, MqttConfig, Settings};

pub struct InterfaceManager {}

pub trait Interface {
    async fn start(&self) -> Result<()>;
}

impl InterfaceManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(
        &self,
        config: &AppConfig,
        control: mpsc::Sender<ControlCommand>,
        state: watch::Sender<ApplicationState>,
        settings: watch::Receiver<Settings>,
    ) -> Result<thread::JoinHandle<Result<()>>> {
        let config = config.clone();
        let bg_thread = std::thread::Builder::new()
            .name("interfaces".to_string())
            .spawn(move || -> Result<()> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .context("Failed to create tokio runtime")?;
                runtime.block_on(async move {
                    let http = async {
                        if let Some(http_config @ HttpConfig { enabled: true, .. }) = config.http {
                            let interface =
                                HttpInterface::new(http_config, settings.clone(), control.clone());
                            interface.start().await?;
                        }
                        Ok::<(), anyhow::Error>(())
                    };
                    let mqtt = async {
                        if let Some(mqtt_config @ MqttConfig { enabled: true, .. }) = config.mqtt {
                            let mqtt = MqttInterface::new(
                                mqtt_config,
                                control.clone(),
                                state.clone(),
                                settings.clone(),
                            );
                            mqtt.start().await?
                        }
                        Ok::<(), anyhow::Error>(())
                    };
                    try_join!(http, mqtt)
                })?;
                Ok(())
            })?;
        Ok(bg_thread)
    }
}
