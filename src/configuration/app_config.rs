use better_default::Default;
use serde::Deserialize;

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub sources: Vec<Source>,
    pub mqtt: Option<MqttConfig>,
    pub http: Option<HttpConfig>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Source {
    Immich(ImmichSource),
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct ImmichSource {
    pub instance: Option<ImmichInstance>,
    pub instances: Vec<ImmichInstance>,
    pub specs: Vec<ImmichSpec>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ImmichInstance {
    pub url: String,
    pub api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ImmichSpec {
    RandomSearch(ImmichSearchQuery),
    SmartSearch(ImmichSmartSearchQuery),
    PrivateAlbum(PrivateAlbum),
    MemoryLane,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PrivateAlbum {
    pub id: String,
}

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ImmichSearchQuery {
    pub persons: Option<Vec<ImmichPerson>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ImmichSmartSearchQuery {
    pub persons: Option<Vec<ImmichPerson>>,
    pub query: String,
    pub city: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct MqttConfig {
    pub enabled: bool,
    #[default("localhost".into())]
    pub host: String,
    #[default(1883)]
    pub port: u16,
    pub credentials: Option<MqttCredentials>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MqttCredentials {
    pub user: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct HttpConfig {
    #[serde(default)]
    pub enabled: bool,

    #[default("0.0.0.0:3000".into())]
    pub bind_address: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ImmichPerson {
    Id(String),
    Name(String),
}
