use std::{cell::RefCell, ops::Deref, time::Duration};

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use log::error;
use rumqttc::v5::{
    mqttbytes::{v5::ConnectReturnCode, QoS},
    AsyncClient, ConnectionError, Event, EventLoop, Incoming, MqttOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::watch, try_join};

use super::Interface;
use crate::configuration::Settings;

pub struct MqttInterface {
    id: String,
}

impl MqttInterface {
    pub fn new() -> Self {
        let id = std::env::var("MQTT_ID").unwrap_or("photokiosk".to_string());
        Self { id }
    }

    fn topic(&self, kind: &str) -> String {
        format!("homeassistant/device/{}/{}", self.id, kind)
    }

    fn command_topic(&self) -> String {
        self.topic("set")
    }

    fn state_topic(&self) -> String {
        self.topic("state")
    }

    fn config_topic(&self) -> String {
        self.topic("config")
    }

    fn component_id(&self, component: &str) -> String {
        format!("{}_{}", self.id, component)
    }

    fn config_payload(&self) -> serde_json::Value {
        let c = |c| self.component_id(c);
        json!({
            "device": {
                "name": "PhotoKiosk",
                "identifiers": [self.id],
            },
            "origin": {
                "name": "PhotoKiosk",
                "sw_version": "0.1.0",
            },
            "components": {
                c("display_duration"): {
                    "p": "number",
                    "device_class": "duration",
                    "unit_of_measurement": "s",
                    "min": 1,
                    "max": 60 * 60 * 24,
                    "name": "Display Duration",
                    "value_template": "{{ value_json.display_duration }}",
                    "command_template": r#"{ "type": "display_duration", "value": {{ value }} }"#,
                    "unique_id": c("display_duration"),
                }
            },
            "command_topic": self.command_topic(),
            "state_topic": self.state_topic(),
        })
    }

    async fn config_send(&self, client: &AsyncClient) -> Result<()> {
        let topic = self.config_topic();
        let payload = self.config_payload();
        client
            .publish(
                &topic,
                QoS::AtLeastOnce,
                true,
                serde_json::to_string(&payload).context("Failed to serialize config payload")?,
            )
            .await
            .context("Failed to publish config")?;
        client
            .subscribe(self.command_topic(), QoS::AtLeastOnce)
            .await
            .context("Failed to subscribe to command topic")?;
        Ok(())
    }

    async fn state_send(
        &self,
        client: &AsyncClient,
        settings: watch::Receiver<Settings>,
    ) -> Result<()> {
        let topic = self.state_topic();
        loop {
            let state = {
                let settings = settings.borrow();
                MqttState::from(settings.deref())
            };
            client
                .publish(
                    &topic,
                    QoS::AtLeastOnce,
                    true,
                    serde_json::to_string(&state).context("Failed to serialize state payload")?,
                )
                .await
                .context("Failed to publish state")?;
        }
    }

    async fn command_receive(
        &self,
        connection: EventLoop,
        settings: watch::Sender<Settings>,
    ) -> Result<()> {
        let command_topic = self.command_topic();
        let poller = RetryPoller::new(connection);
        loop {
            let n = poller.poll().await.context("Failed to poll mqtt")?;
            if let Event::Incoming(Incoming::Publish(publish)) = n {
                if publish.topic != command_topic {
                    continue;
                }
                println!("Received command");

                let message: MqttMessage = match serde_json::from_slice(&publish.payload) {
                    Ok(m) => m,
                    Err(err) => {
                        error!("Failed to parse incoming message: {}", err);
                        continue;
                    }
                };
                println!("Message: {:?}", message);
                match message {
                    MqttMessage::DisplayDuration(duration) => {
                        let duration = Duration::from_secs(duration);
                        settings.send_modify(|s| {
                            s.display_duration = duration;
                        });
                    }
                }
            }
        }
    }
}

struct RetryPoller {
    connection: RefCell<EventLoop>,
}

impl RetryPoller {
    fn new(connection: EventLoop) -> Self {
        Self {
            connection: RefCell::new(connection),
        }
    }

    async fn poll(&self) -> Result<Event> {
        let event = (|| async { self.connection.borrow_mut().poll().await })
            .retry(ExponentialBuilder::default())
            .sleep(tokio::time::sleep)
            .when(Self::is_recoverable)
            .await
            .context("Unrecoverable MQTT error")?;
        Ok(event)
    }

    fn is_recoverable(err: &ConnectionError) -> bool {
        match err {
            ConnectionError::ConnectionRefused(
                ConnectReturnCode::ProtocolError
                | ConnectReturnCode::UnsupportedProtocolVersion
                | ConnectReturnCode::ClientIdentifierNotValid
                | ConnectReturnCode::BadUserNamePassword
                | ConnectReturnCode::NotAuthorized
                | ConnectReturnCode::Banned
                | ConnectReturnCode::BadAuthenticationMethod
                | ConnectReturnCode::UseAnotherServer
                | ConnectReturnCode::ServerMoved,
            ) => false,
            _ => true,
        }
    }
}

#[derive(Debug, Serialize)]
struct MqttState {
    display_duration: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum MqttMessage {
    DisplayDuration(u64),
}

impl From<&Settings> for MqttState {
    fn from(settings: &Settings) -> Self {
        MqttState {
            display_duration: settings.display_duration.as_secs(),
        }
    }
}

impl Interface for MqttInterface {
    async fn start(&self, settings: watch::Sender<Settings>) -> Result<()> {
        let mut mqtt_options = MqttOptions::new("rumqtt", "192.168.1.18", 1883);
        let user = std::env::var("MQTT_USER")?;
        let password = std::env::var("MQTT_PASSWORD")?;
        mqtt_options.set_credentials(user, password);
        let (client, connection) = AsyncClient::new(mqtt_options, 10);

        try_join!(
            self.state_send(&client, settings.subscribe()),
            self.config_send(&client),
            self.command_receive(connection, settings),
        )
        .context("in MQTT interface")?;
        Ok(())
    }
}
