use std::time::Duration;

use better_default::Default;
use chrono::Locale;
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use struct_patch::Patch;

#[derive(Deserialize, Serialize, Debug, Clone, Default, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct BlurSettings {
    #[default(6.0)]
    pub radius: f32,
    #[default(3)]
    pub passes: u8,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    /// The minimum amount of time that photos are displayed before switching to the next.
    ///
    /// Please note that on low-power devices, photos may be displayed for longer
    /// than this minimum duration if the next photo is not yet available.
    /// Defaults to 30 seconds ("30s").
    #[default(Duration::from_secs(30))]
    #[serde(with = "humantime_serde")]
    #[patch(attribute(serde(with = "humantime_serde")))]
    pub display_duration: Duration,

    /// Duration of the transition between two photos.
    /// Defaults to 1 second ("500ms").
    #[default(Duration::from_millis(500))]
    #[serde(with = "humantime_serde")]
    #[patch(attribute(serde(with = "humantime_serde")))]
    pub transition_duration: Duration,

    /// The options for the initial slide.
    /// Defaults to a loading circle.
    /// Possible values are "empty" and "loading-circle".
    pub init_slide: InitSlideOptions,

    /// The options for the blur effect.
    #[patch(name = "BlurSettingsPatch")]
    pub blur_options: BlurSettings,

    /// The options for the background, aka the area around the photos when they don't fill the screen.
    /// Defaults to a blurred version of the photo.
    /// Possible values are "black" and "blur".
    pub background: Background,

    /// The orientation of the display.
    /// Defaults to 0 degrees.
    /// Possible values are 0, 90, 180, 270.
    pub rotation: OrientationName,

    /// The options for the caption (photo information displayed at the bottom of the screen).
    #[patch(name = "CaptionOptionsPatch")]
    pub caption: CaptionOptions,

    /// Photos larger than the display are downscaled using this filter.
    pub downscaled_image_filter: ImageFilter,

    /// The options for the debug overlay.
    #[patch(name = "DebugSettingsPatch")]
    pub debug: DebugSettings,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum ImageFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    #[default]
    Lanczos3,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct DebugSettings {
    pub show_fps: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct CaptionOptions {
    /// Whether the caption is enabled.
    #[default(true)]
    pub enabled: bool,

    /// The format of the date in the caption.
    #[patch(name = "DateFormatPatch")]
    pub date_format: DateFormat,

    /// The font size of the caption.
    #[default(28.)]
    pub font_size: f32,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct DateFormat {
    /// The format of the date in the caption.
    /// Defaults to "%A, %e. %B %Y".
    /// See https://docs.rs/chrono/0.4.39/chrono/format/strftime/index.html for more information.
    #[default("%A, %e. %B %Y".into())]
    pub format: String,

    /// The locale to use for the date.
    /// Defaults to "en_US".
    pub locale: ConfigLocale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigLocale(pub Locale);

impl Default for ConfigLocale {
    fn default() -> Self {
        ConfigLocale(Locale::en_US)
    }
}

impl Serialize for ConfigLocale {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ser.serialize_str(&self.0.to_string())
    }
}

impl<'d> Deserialize<'d> for ConfigLocale {
    fn deserialize<D>(deser: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deser)?;
        s.parse()
            .map(ConfigLocale)
            .map_err(|e| serde::de::Error::custom(format!("Invalid locale: {:?}", e)))
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum Background {
    Black,
    #[default]
    Blur(BlurBackground),
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
#[serde(deny_unknown_fields, default)]
pub struct BlurBackground {
    #[default(50)]
    pub min_free_space: u16,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum InitSlideOptions {
    Empty,
    #[default]
    LoadingCircle(LoadingCircleOptions),
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq, Patch)]
#[patch(attribute(derive(Debug, Default, Deserialize, Serialize)))]
#[patch(attribute(serde(default)))]
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
