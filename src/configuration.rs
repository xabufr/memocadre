use std::time::Duration;

use chrono::Locale;
use serde::{Deserialize, Deserializer};
use serde_repr::Deserialize_repr;

use crate::graphics::BlurOptions;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Conf {
    pub sources: Vec<Source>,
    pub slideshow: Slideshow,
    #[serde(default)]
    pub debug: DebugSettings,
}

#[cfg(test)]
impl Conf {
    pub fn mock() -> Self {
        Self {
            sources: Default::default(),
            slideshow: Default::default(),
            debug: Default::default(),
        }
    }
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

    pub init_slide: InitSlideOptions,
    pub blur_options: BlurOptions,
    pub background: Background,
    pub rotation: OrientationName,
    pub caption: CaptionOptions,
    pub downscaled_image_filter: ImageFilter,
}

#[derive(Deserialize, Debug, Default, Clone, Copy)]
pub enum ImageFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    #[default]
    Lanczos3,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct DebugSettings {
    pub show_fps: bool,
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct CaptionOptions {
    pub enabled: bool,
    pub date_format: DateFormat,
    pub font_size: f32,
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct DateFormat {
    pub format: String,
    #[serde(deserialize_with = "deser_locale")]
    pub locale: Locale,
}
fn deser_locale<'de, D>(deser: D) -> Result<Locale, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deser)?;
    s.parse()
        .map_err(|e| serde::de::Error::custom(format!("Invalid locale: {:?}", e)))
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

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum InitSlideOptions {
    Empty,
    LoadingCircle(LoadingCircleOptions),
}

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct LoadingCircleOptions {
    /// Number of rotations per second.
    pub velocity: f32,
}

#[derive(Clone, Copy, Deserialize_repr, Debug, Default)]
#[serde(deny_unknown_fields)]
#[repr(u16)]
pub enum OrientationName {
    #[default]
    Angle0 = 0,
    Angle90 = 90,
    Angle180 = 180,
    Angle270 = 270,
}

impl Default for Slideshow {
    fn default() -> Self {
        Self {
            background: Background::default(),
            blur_options: BlurOptions::default(),
            init_slide: Default::default(),
            display_duration: Duration::from_secs(30),
            transition_duration: Duration::from_secs(1),
            rotation: Default::default(),
            caption: Default::default(),
            downscaled_image_filter: Default::default(),
        }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::Burr { min_free_space: 50 }
    }
}

impl Default for DateFormat {
    fn default() -> Self {
        Self {
            format: "%A, %e. %B %Y".to_string(),
            locale: Locale::en_US,
        }
    }
}

impl Default for CaptionOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            date_format: Default::default(),
            font_size: 28.,
        }
    }
}

impl Default for InitSlideOptions {
    fn default() -> Self {
        Self::LoadingCircle(Default::default())
    }
}

impl Default for LoadingCircleOptions {
    fn default() -> Self {
        Self { velocity: 1.5 }
    }
}
