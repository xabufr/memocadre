use anyhow::Result;
use axum::{
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use tokio::sync::watch;

use super::Interface;
use crate::configuration::Settings;

pub struct HttpInterface;

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
