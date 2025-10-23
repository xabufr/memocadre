use std::{cell::RefCell, ops::Deref, sync::mpsc, time::Duration};

use anyhow::{Context, Result};
use backon::{ExponentialBuilder, Retryable};
use log::{debug, error, info, warn};
use rumqttc::v5::{
    mqttbytes::{
        v5::{ConnAck, ConnectReturnCode, Publish},
        QoS,
    },
    AsyncClient, ConnectionError, Event, EventLoop, Incoming, MqttOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::watch, try_join};

use super::Interface;
use crate::{
    application::{ApplicationState, ControlCommand},
    configuration::{MqttConfig, Settings, SettingsPatch},
};

pub struct MqttInterface {
    id: String,
    config: MqttConfig,

    control: mpsc::Sender<ControlCommand>,
    state: watch::Sender<ApplicationState>,
    settings: watch::Receiver<Settings>,
}

impl MqttInterface {
    pub fn new(
        config: MqttConfig,
        control: mpsc::Sender<ControlCommand>,
        state: watch::Sender<ApplicationState>,
        settings: watch::Receiver<Settings>,
    ) -> Self {
        let id = std::env::var("MQTT_ID").unwrap_or_else(|_| match machine_uid::get() {
            Ok(id) => id,
            Err(err) => {
                let def = "photokiosk".to_string();
                warn!("Failed to get machine id: {}, defaulting to {}", err, def);
                def
            }
        });
        Self {
            id,
            config,
            control,
            state,
            settings,
        }
    }

    fn topic(&self, kind: &str) -> String {
        format!("homeassistant/device/photokiosk_{}/{}", self.id, kind)
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
                "name": format!("PhotoKiosk {}", self.id),
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
                },
                c("display_enabled"): {
                    "p": "switch",
                    "name": "Display Enabled",
                    "value_template": r#"{{ "ON" if value_json.display_enabled else "OFF" }}"#,
                    "command_template": r#"{ "type": "display_enabled", "value": {{ "true" if value == "ON" else "false" }} }"#,
                    "unique_id": c("display_enabled"),
                },
                c("next"): {
                    "p": "button",
                    "name": "Next photo",
                    "command_template": r#"{ "type": "next_slide" }"#,
                    "unique_id": c("next"),
                },
            },
            "command_topic": self.command_topic(),
            "state_topic": self.state_topic(),
        })
    }

    fn try_send_config_and_subscribe(&self, client: &AsyncClient) -> Result<()> {
        let topic = self.config_topic();
        let payload = self.config_payload();
        client
            .try_publish(
                &topic,
                QoS::AtLeastOnce,
                true,
                serde_json::to_string(&payload).context("Failed to serialize config payload")?,
            )
            .context("Failed to publish config")?;
        client
            .try_subscribe(self.command_topic(), QoS::AtLeastOnce)
            .context("Failed to subscribe to command topic")?;
        Ok(())
    }

    async fn state_send(&self, client: &AsyncClient) -> Result<()> {
        let mut state = self.state.subscribe();
        let mut settings = self.settings.clone();
        let topic = self.state_topic();
        loop {
            let mqtt_state = {
                let settings = settings.borrow_and_update();
                let state = state.borrow_and_update();
                MqttState::from((settings.deref(), state.deref()))
            };
            client
                .publish(
                    &topic,
                    QoS::AtLeastOnce,
                    true,
                    serde_json::to_string(&mqtt_state)
                        .context("Failed to serialize state payload")?,
                )
                .await
                .context("Failed to publish state")?;
            tokio::select! {
                _ = state.changed() => {},
                _ = settings.changed() => {},
            }
        }
    }

    async fn command_receive(&self, client: &AsyncClient, connection: EventLoop) -> Result<()> {
        let command_topic = self.command_topic();
        let poller = RetryPoller::new(connection);
        loop {
            let n = poller.poll().await.context("Failed to poll mqtt")?;
            match n {
                Event::Incoming(Incoming::ConnAck(ConnAck {
                    code: ConnectReturnCode::Success,
                    ..
                })) => {
                    self.try_send_config_and_subscribe(client)
                        .context("Initializing MQTT resources")?;
                }
                Event::Incoming(Incoming::Publish(publish)) => {
                    if publish.topic != command_topic {
                        continue;
                    }
                    self.handle_mqtt_message(publish)
                        .await
                        .context("Error when processing MQTT message")?
                }
                _ => {}
            }
        }
    }

    async fn handle_mqtt_message(&self, publish: Publish) -> Result<()> {
        let message: MqttMessage = match serde_json::from_slice(&publish.payload) {
            Ok(m) => m,
            Err(err) => {
                error!("Failed to parse incoming message: {}", err);
                return Ok(());
            }
        };
        debug!("MQTT Message: {:?}", message);
        match message {
            MqttMessage::DisplayDuration(duration) => {
                let duration = Duration::from_secs(duration);
                self.control
                    .send(ControlCommand::ConfigChanged(SettingsPatch {
                        display_duration: Some(duration),
                        ..Default::default()
                    }))
                    .context("Failed to send control command")?;
            }
            MqttMessage::DisplayEnabled(false) => {
                self.control
                    .send(ControlCommand::DisplayOff)
                    .context("Failed to send control command")?;
            }
            MqttMessage::DisplayEnabled(true) => {
                self.control
                    .send(ControlCommand::DisplayOn)
                    .context("Failed to send control command")?;
            }
            MqttMessage::NextSlide => {
                self.control
                    .send(ControlCommand::NextSlide)
                    .context("Failed to send control command")?;
            }
        }
        Ok(())
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
        // This is safe because the connection is only borrowed for the duration of the poll
        // and the poll method is not called again until the connection is returned
        #[allow(clippy::await_holding_refcell_ref)]
        let event = (|| async { self.connection.borrow_mut().poll().await })
            .retry(
                ExponentialBuilder::default()
                    .without_max_times()
                    .with_max_delay(Duration::from_secs(10)),
            )
            .sleep(tokio::time::sleep)
            .when(Self::is_recoverable)
            .notify(|error, sleep| {
                warn!("Recoverable MQTT error: {error}, will retry in {sleep:?}");
            })
            .await
            .context("Unrecoverable MQTT error")?;
        Ok(event)
    }

    fn is_recoverable(err: &ConnectionError) -> bool {
        !matches!(
            err,
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
            )
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_recoverable() {
        let err = ConnectionError::ConnectionRefused(ConnectReturnCode::ServerMoved);
        assert_eq!(false, RetryPoller::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_io_error() {
        let err = ConnectionError::Io(std::io::ErrorKind::HostUnreachable.into());
        assert_eq!(true, RetryPoller::is_recoverable(&err));
    }
}

#[derive(Debug, Serialize)]
struct MqttState {
    display_duration: u64,
    display_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum MqttMessage {
    DisplayDuration(u64),
    DisplayEnabled(bool),
    NextSlide,
}

impl From<(&Settings, &ApplicationState)> for MqttState {
    fn from(state: (&Settings, &ApplicationState)) -> Self {
        MqttState {
            display_duration: state.0.display_duration.as_secs(),
            display_enabled: state.1.display,
        }
    }
}

impl Interface for MqttInterface {
    async fn start(&self) -> Result<()> {
        info!("Starting MQTT interface");
        let mut mqtt_options = MqttOptions::new(
            format!("photokiosk_{}", self.id),
            &self.config.host,
            self.config.port,
        );
        mqtt_options.set_clean_start(false);
        if let Some(creds) = &self.config.credentials {
            mqtt_options.set_credentials(&creds.username, &creds.password);
        }
        let (client, connection) = AsyncClient::new(mqtt_options, 10);

        try_join!(
            self.state_send(&client),
            self.command_receive(&client, connection),
        )
        .context("in MQTT interface")?;
        Ok(())
    }
}
