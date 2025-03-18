use std::sync::mpsc;

use anyhow::{Context, Result};
use axum::{
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use log::info;
use tokio::sync::watch;

use super::Interface;
use crate::{
    application::{ApplicationState, ControlCommand},
    configuration::{HttpConfig, Settings},
};

pub struct HttpInterface {
    config: HttpConfig,
}

impl HttpInterface {
    pub fn new(config: HttpConfig) -> Self {
        Self { config }
    }
}

impl Interface for HttpInterface {
    async fn start(
        &self,
        _control: mpsc::Sender<ControlCommand>,
        _state: watch::Sender<ApplicationState>,
        settings: watch::Sender<Settings>,
    ) -> Result<()> {
        info!("Starting HTTP interface");
        let app = Router::new()
            .route(
                "/settings",
                get({
                    let settings = settings.clone();
                    || async move {
                        let settings = settings.borrow().clone();
                        Json::from(settings)
                    }
                }),
            )
            .route(
                "/settings",
                put({
                    let settings = settings.clone();
                    |new_settings: Json<Settings>| async move {
                        settings.send_replace(new_settings.0);
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
