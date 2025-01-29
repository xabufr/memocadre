use std::time::Duration;

use serde::Deserialize;

use crate::graphics::BlurOptions;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Conf {
    pub sources: Vec<Source>,
    pub slideshow: Slideshow,
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Slideshow {
    /// The minimum amount of time that photos are displayed before switching to the next.
    ///
    /// Please note that on low-power devices, photos may be displayed for longer
    /// than this minimum duration if the next photo is not yet available.
    #[serde(with = "humantime_serde")]
    pub display_duration: Duration,

    /// Duration of the transition between two photos.
    #[serde(with = "humantime_serde")]
    pub transition_duration: Duration,

    #[serde(default)]
    pub blur_options: BlurOptions,
    #[serde(default)]
    pub background: Background,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Background {
    Black,
    Burr { min_free_space: u16 },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    Immich(ImmichSource),
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichSource {
    #[serde(default, flatten)]
    pub instance: Option<ImmichInstance>,
    #[serde(default)]
    pub instances: Vec<ImmichInstance>,
    pub specs: Vec<ImmichSpec>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichInstance {
    pub url: String,
    pub api_key: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum ImmichSpec {
    RandomSearch(ImmichSearchQuery),
    SmartSearch(ImmichSmartSearchQuery),
    PrivateAlbum { id: String },
    MemoryLane,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichSearchQuery {
    #[serde(default)]
    pub persons: Option<Vec<ImmichPerson>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichSmartSearchQuery {
    #[serde(default)]
    pub persons: Option<Vec<ImmichPerson>>,
    pub query: String,
    #[serde(default)]
    pub city: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum ImmichPerson {
    Id(String),
    Name(String),
}

impl Default for Slideshow {
    fn default() -> Self {
        Self {
            background: Background::default(),
            blur_options: BlurOptions::default(),
            display_duration: Duration::from_secs(30),
            transition_duration: Duration::from_secs(1),
        }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::Burr { min_free_space: 50 }
    }
}
