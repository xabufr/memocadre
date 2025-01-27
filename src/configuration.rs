use std::time::Duration;

use serde::Deserialize;

use crate::graphics::BlurOptions;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Conf {
    pub source: ImmichSource,
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

#[derive(Deserialize, Debug)]
pub struct ImmichSource {
    pub url: String,
    pub api_key: String,
    #[serde(default)]
    pub search: Option<ImmichSearchQuery>,
}

#[derive(Deserialize, Debug)]
pub struct ImmichSearchQuery {
    #[serde(default)]
    pub persons: Option<Vec<ImmichPerson>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ImmichPerson {
    Id(String),
    Name(String),
}
