use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Conf {
    pub source: ImmichSource,
    pub slideshow: Slideshow,
}

#[derive(Deserialize, Debug)]
pub struct Slideshow {
    /// The minimum amount of time that photos are displayed before switching to the next.
    ///
    /// Please note that on low-power devices, photos may be displayed for longer
    /// than this minimum duration if the next photo is not yet available.
    #[serde(with = "humantime_serde", default = "default_display_duration")]
    pub display_duration: Duration,
}

fn default_display_duration() -> Duration {
    Duration::from_secs(30)
}

#[derive(Deserialize, Debug)]
pub struct ImmichSource {
    pub url: String,
    pub api_key: String,
}
