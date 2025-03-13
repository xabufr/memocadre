use std::time::Duration;

use anyhow::Result;
use rumqttc::v5::{mqttbytes::QoS, AsyncClient, Incoming, MqttOptions, Event};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::watch;

use super::Interface;
use crate::configuration::Settings;

pub struct MqttInterface;

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
        let id = std::env::var("MQTT_ID").unwrap_or("photokiosk".to_string());
        let mut mqtt_options = MqttOptions::new("rumqtt", "192.168.1.18", 1883);
        let user = std::env::var("MQTT_USER")?;
        let password = std::env::var("MQTT_PASSWORD")?;
        mqtt_options.set_credentials(user, password);
        let (client, mut connection) = AsyncClient::new(mqtt_options, 10);

        let command_topic = format!("homeassistant/device/{id}/set");
        let payload = json!({
            "device": {
                "name": "PhotoKiosk",
                "identifiers": [id],
            },
            "origin": {
                "name": "PhotoKiosk",
                "sw_version": "0.1.0",
            },
            "components": {
                format!("{id}_display_time"): {
                    "p": "number",
                    "device_class": "duration",
                    "unit_of_measurement": "s",
                    "min": 1,
                    "max": 60 * 60 * 24,
                    "name": "Display Duration",
                    "value_template": "{{ value_json.display_duration }}",
                    "command_template": r#"{ "type": "display_duration", "value": {{ value }} }"#,
                    "unique_id": format!("{id}_display_time"),
                }
            },
            "command_topic": &command_topic,
            "state_topic": format!("homeassistant/device/{id}/state"),
        });
        client.subscribe(&command_topic, QoS::AtMostOnce).await?;

        tokio::spawn({
            let id = id.clone();
            let client = client.clone();
            async move {
                println!("Sending...");
                client
                    .publish(
                        format!("homeassistant/device/{id}/config"),
                        QoS::AtLeastOnce,
                        true,
                        serde_json::to_string(&payload).unwrap(),
                    )
                    .await
                    .unwrap();
                println!("Sent!");
            }
        });
        tokio::spawn({
            let id = id.clone();
            let client = client.clone();
            let mut settings = settings.subscribe();
            async move {
                loop {
                    let state = {
                        let settings = settings.borrow();
                        MqttState::from(&*settings)
                    };
                    client
                        .publish(
                            format!("homeassistant/device/{id}/state"),
                            QoS::AtLeastOnce,
                            true,
                            serde_json::to_string(&state).unwrap(),
                        )
                        .await
                        .unwrap();
                    settings.changed().await.unwrap();
                }
            }
        });
        loop {
            let n = connection.poll().await?;
            println!("Polling: {n:?}");
            if let Event::Incoming(Incoming::Publish(publish)) = n {
                if publish.topic !=command_topic {
                    continue;
                }
                println!("Received command");

                // FIXME ignore errors
                let message: MqttMessage = serde_json::from_slice(&publish.payload)?;
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

        // Ok(())
    }
}
