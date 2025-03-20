use std::sync::mpsc;

use anyhow::{Context, Result};
use axum::{
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use log::info;
use tokio::sync::watch;

use super::Interface;
use crate::{
    application::ControlCommand,
    configuration::{HttpConfig, Settings, SettingsPatch},
};

pub struct HttpInterface {
    config: HttpConfig,
    control: mpsc::Sender<ControlCommand>,
    settings: watch::Receiver<Settings>,
}

impl HttpInterface {
    pub fn new(
        config: HttpConfig,
        settings: watch::Receiver<Settings>,
        control: mpsc::Sender<ControlCommand>,
    ) -> Self {
        Self {
            config,
            settings,
            control,
        }
    }
}

impl Interface for HttpInterface {
    async fn start(&self) -> Result<()> {
        info!("Starting HTTP interface");
        let app = Router::new()
            .route(
                "/settings",
                get({
                    let settings = self.settings.clone();
                    || async move {
                        let settings = settings.borrow().clone();
                        Json::from(settings)
                    }
                }),
            )
            .route(
                "/settings",
                patch({
                    let control = self.control.clone();
                    async move |settings_patch: Json<SettingsPatch>| {
                        control.send(ControlCommand::ConfigChanged(settings_patch.0)).map_err(|err| {
                            log::error!("Failed to send control command: {}", err);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })
                    }
                }),
            )
            .fallback(|| async { StatusCode::NOT_FOUND });

        let listener = tokio::net::TcpListener::bind(&self.config.bind_address)
            .await
            .context("Failed to bind to address")?;
        axum::serve(listener, app)
            .await
            .context("Failed to start HTTP server")?;
        Ok(())
    }
}
