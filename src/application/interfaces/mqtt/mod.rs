use anyhow::Result;
use rumqttc::v5::{mqttbytes::QoS, AsyncClient, MqttOptions};
use serde::Serialize;
use serde_json::json;
use tokio::sync::watch;

use super::Interface;
use crate::configuration::Settings;

pub struct MqttInterface;

#[derive(Debug, Serialize)]
struct MqttState {
    display_duration: u64,
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
        let mut mqttOptions = MqttOptions::new("rumqtt", "192.168.1.18", 1883);
        let user = std::env::var("MQTT_USER")?;
        let password = std::env::var("MQTT_PASSWORD")?;
        mqttOptions.set_credentials(user, password);
        let (mut client, mut connection) = AsyncClient::new(mqttOptions, 10);

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
                    "min": 30,
                    "max": 60 * 60 * 24,
                    "name": "Display Duration",
                    "value_template": "{{ value_json.display_duration }}",
                    "unique_id": format!("{id}_display_time"),
                }
            },
            "command_topic": format!("homeassistant/device/{id}/set"),
            "state_topic": format!("homeassistant/device/{id}/state"),
        });

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
            async move {
                let mut settings = settings.subscribe();
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
        }

        // Ok(())
    }
}
