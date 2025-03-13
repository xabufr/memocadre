mod mqtt;

use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use mqtt::MqttInterface;
use tokio::{sync::watch, try_join};

use crate::configuration::Settings;

pub struct InterfaceManager {}

pub trait Interface {
    async fn start(&self, settings: watch::Sender<Settings>) -> Result<()>;
}

struct HttpInterface;

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

impl Interface for HttpInterface {
    async fn start(&self, settings: watch::Sender<Settings>) -> Result<()> {
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

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
        Ok(())
    }
}
