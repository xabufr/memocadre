use anyhow::Result;
use axum::{
    http::{Response, StatusCode},
    routing::get,
    Json, Router,
};
use tokio::sync::watch;

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
            runtime.block_on(async move {
                interface.start(settings).await.unwrap();
            });
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
                    let settings = settings.borrow().clone();
                    move || async { Json::from(settings) }
                }),
            )
            .fallback(|| async { StatusCode::NOT_FOUND });

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
        Ok(())
    }
}
