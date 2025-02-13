use better_default::Default;
use chrono::Locale;
use serde::{Deserialize, Deserializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::time::Duration;

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct AppConfiguration {
    pub sources: Vec<Source>,
    #[serde(default)]
    pub slideshow: Slideshow,
    #[serde(default)]
    pub debug: DebugSettings,
}

#[cfg(test)]
impl AppConfiguration {
    pub fn mock() -> Self {
        Self {
            sources: Default::default(),
            slideshow: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct BlurConfig {
    #[default(6.0)]
    pub radius: f32,
    #[default(3)]
    pub passes: u8,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Slideshow {
    /// The minimum amount of time that photos are displayed before switching to the next.
    ///
    /// Please note that on low-power devices, photos may be displayed for longer
    /// than this minimum duration if the next photo is not yet available.
    /// Defaults to 30 seconds ("30s").
    #[default(Duration::from_secs(30))]
    #[serde(with = "humantime_serde")]
    pub display_duration: Duration,

    /// Duration of the transition between two photos.
    /// Defaults to 1 second ("1s").
    #[default(Duration::from_secs(1))]
    #[serde(with = "humantime_serde")]
    pub transition_duration: Duration,

    /// The options for the initial slide.
    /// Defaults to a loading circle.
    /// Possible values are "empty" and "loading-circle".
    pub init_slide: InitSlideOptions,

    /// The options for the blur effect.
    pub blur_options: BlurConfig,

    /// The options for the background, aka the area around the photos when they don't fill the screen.
    /// Defaults to a blurred version of the photo.
    /// Possible values are "black" and "blur".
    pub background: Background,

    /// The orientation of the display.
    /// Defaults to 0 degrees.
    /// Possible values are 0, 90, 180, 270.
    pub rotation: OrientationName,

    /// The options for the caption (photo information displayed at the bottom of the screen).
    pub caption: CaptionOptions,

    /// Photos larger than the display are downscaled using this filter.
    pub downscaled_image_filter: ImageFilter,
}

#[derive(Deserialize, Debug, Copy, Clone, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ImageFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    #[default]
    Lanczos3,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, default)]
pub struct DebugSettings {
    pub show_fps: bool,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct CaptionOptions {
    /// Whether the caption is enabled.
    #[default(true)]
    pub enabled: bool,

    /// The format of the date in the caption.
    pub date_format: DateFormat,

    /// The font size of the caption.
    #[default(28.)]
    pub font_size: f32,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct DateFormat {
    /// The format of the date in the caption.
    /// Defaults to "%A, %e. %B %Y".
    /// See https://docs.rs/chrono/0.4.39/chrono/format/strftime/index.html for more information.
    #[default("%A, %e. %B %Y".into())]
    pub format: String,

    /// The locale to use for the date.
    /// Defaults to "en_US".
    #[default(Locale::en_US)]
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
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Background {
    Black,
    #[default]
    Blur(BlurBackground),
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct BlurBackground {
    #[default(50)]
    pub min_free_space: u16,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Source {
    Immich(ImmichSource),
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct ImmichSource {
    pub instance: Option<ImmichInstance>,
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
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum ImmichSpec {
    RandomSearch(ImmichSearchQuery),
    SmartSearch(ImmichSmartSearchQuery),
    PrivateAlbum(PrivateAlbum),
    MemoryLane,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PrivateAlbum {
    pub id: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichSearchQuery {
    pub persons: Option<Vec<ImmichPerson>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ImmichSmartSearchQuery {
    pub persons: Option<Vec<ImmichPerson>>,
    pub query: String,
    pub city: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ImmichPerson {
    Id(String),
    Name(String),
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum InitSlideOptions {
    Empty,
    #[default]
    LoadingCircle(LoadingCircleOptions),
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct LoadingCircleOptions {
    /// Number of rotations per second for the circle.
    #[default(1.5)]
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
