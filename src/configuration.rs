use std::time::Duration;

use chrono::Locale;
use schematic::{derive_enum, Config, ConfigEnum};
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Config, Debug)]
pub struct Conf {
    #[setting(nested)]
    pub sources: Vec<Source>,
    #[setting(nested)]
    pub slideshow: Slideshow,
    #[setting(nested)]
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

#[derive(Config, Debug)]
pub struct BlurOptions {
    radius: f32,
    passes: u8,
}

#[derive(Config, Debug)]
pub struct Slideshow {
    /// The minimum amount of time that photos are displayed before switching to the next.
    ///
    /// Please note that on low-power devices, photos may be displayed for longer
    /// than this minimum duration if the next photo is not yet available.
    // #[serde(with = "humantime_serde")]
    pub display_duration: Duration,

    /// Duration of the transition between two photos.
    // #[serde(with = "humantime_serde")]
    pub transition_duration: Duration,

    #[setting(nested)]
    pub init_slide: InitSlideOptions,
    #[setting(nested)]
    pub blur_options: BlurOptions,
    #[setting(nested)]
    pub background: Background,
    // #[setting(nested)]
    pub rotation: OrientationName,
    #[setting(nested)]
    pub caption: CaptionOptions,
    pub downscaled_image_filter: ImageFilter,
}

derive_enum! {
    #[derive(ConfigEnum, Copy, Default)]
    pub enum ImageFilter {
        Nearest,
        Triangle,
        CatmullRom,
        Gaussian,
        #[default]
        Lanczos3,
    }
}

#[derive(Config, Debug)]
pub struct DebugSettings {
    pub show_fps: bool,
}

#[derive(Config, Debug)]
pub struct CaptionOptions {
    #[setting(default = true)]
    pub enabled: bool,
    #[setting(nested)]
    pub date_format: DateFormat,
    #[setting(default = 28.)]
    pub font_size: f32,
}

#[derive(Config, Debug)]
pub struct DateFormat {
    #[setting(default = "%A, %e. %B %Y")]
    pub format: String,
    // #[serde(deserialize_with = "deser_locale")]
    pub locale: LocaleWrapper,
}
#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct LocaleWrapper(
    #[serde(deserialize_with = "deser_locale", serialize_with = "ser_locale")] pub Locale,
);
fn deser_locale<'de, D>(deser: D) -> Result<Locale, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deser)?;
    s.parse()
        .map_err(|e| serde::de::Error::custom(format!("Invalid locale: {:?}", e)))
}
fn ser_locale<S>(locale: &Locale, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    locale.to_string().serialize(ser)
}
#[derive(Config, Debug)]
pub enum Background {
    Black,
    #[setting(default, nested)]
    Burr(BlurBackground),
}

#[derive(Config, Debug)]
pub struct BlurBackground {
    #[setting(default = 50)]
    pub min_free_space: u16,
}

#[derive(Config, Debug)]
#[config(serde(tag = "type"))]
pub enum Source {
    #[setting(nested)]
    Immich(ImmichSource),
}

#[derive(Config, Debug)]
pub struct ImmichSource {
    #[setting(nested)]
    pub instance: Option<ImmichInstance>,
    #[setting(nested)]
    pub instances: Vec<ImmichInstance>,
    #[setting(nested)]
    pub specs: Vec<ImmichSpec>,
}

#[derive(Config, Debug)]
pub struct ImmichInstance {
    pub url: String,
    pub api_key: String,
}

#[derive(Config, Debug)]
#[config(serde(tag = "type"))]
pub enum ImmichSpec {
    #[setting(nested)]
    RandomSearch(ImmichSearchQuery),
    #[setting(nested)]
    SmartSearch(ImmichSmartSearchQuery),
    #[setting(nested)]
    PrivateAlbum(PrivateAlbum),
    MemoryLane,
}

#[derive(Config, Debug)]
pub struct PrivateAlbum {
    pub id: String,
}

#[derive(Config, Debug)]
pub struct ImmichSearchQuery {
    #[setting(nested)]
    pub persons: Option<Vec<ImmichPerson>>,
}

#[derive(Config, Debug)]
pub struct ImmichSmartSearchQuery {
    #[setting(nested)]
    pub persons: Option<Vec<ImmichPerson>>,
    pub query: String,
    pub city: Option<String>,
}

#[derive(Config, Debug)]
pub enum ImmichPerson {
    Id(String),
    Name(String),
}

#[derive(Config, Debug)]
#[config(serde(tag = "type"))]
pub enum InitSlideOptions {
    Empty,
    #[setting(nested, default)]
    LoadingCircle(LoadingCircleOptions),
}

#[derive(Config, Debug)]
pub struct LoadingCircleOptions {
    /// Number of rotations per second.
    #[setting(default = 1.5)]
    pub velocity: f32,
}

#[derive(Clone, Copy, Deserialize_repr, Debug, Default, PartialEq, Serialize_repr)]
#[serde(deny_unknown_fields)]
#[repr(u16)]
pub enum OrientationName {
    #[default]
    Angle0 = 0,
    Angle90 = 90,
    Angle180 = 180,
    Angle270 = 270,
}

// impl Default for Slideshow {
//     fn default() -> Self {
//         Self {
//             background: Background::default(),
//             blur_options: BlurOptions::default(),
//             init_slide: Default::default(),
//             display_duration: Duration::from_secs(30),
//             transition_duration: Duration::from_secs(1),
//             rotation: Default::default(),
//             caption: Default::default(),
//             downscaled_image_filter: Default::default(),
//         }
//     }
// }

// impl Default for Background {
//     fn default() -> Self {
//         Self::Burr { min_free_space: 50 }
//     }
// }

impl Default for LocaleWrapper {
    fn default() -> Self {
        LocaleWrapper(Locale::en_US)
    }
}
// impl Default for DateFormat {
//     fn default() -> Self {
//         Self {
//             format: "%A, %e. %B %Y".to_string(),
//             locale: Locale::en_US,
//         }
//     }
// }

// impl Default for CaptionOptions {
//     fn default() -> Self {
//         Self {
//             enabled: true,
//             date_format: Default::default(),
//             font_size: 28.,
//         }
//     }
// }
