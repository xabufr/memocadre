use std::time::{Duration, Instant};

use glissade::Easing;

use super::slide::AnimatedSlideProperties;

pub trait Transition {
    fn ease_in(&self, time: Instant, duration: Duration) -> AnimatedSlideProperties;
    fn ease_out(&self, time: Instant, duration: Duration, properties: &mut AnimatedSlideProperties);
}

pub struct DissolveTransition;

pub struct EaseInOutTransition;

impl Transition for DissolveTransition {
    fn ease_in(&self, time: Instant, duration: Duration) -> AnimatedSlideProperties {
        let mut properties = AnimatedSlideProperties::default();
        properties.set_global_opacity_no_ease(0.0);
        properties.ease_global_opacity(1.0, time, duration, Easing::QuadraticInOut);
        properties
    }

    fn ease_out(
        &self,
        time: Instant,
        duration: Duration,
        properties: &mut AnimatedSlideProperties,
    ) {
        properties.ease_global_opacity(0.0, time, duration, Easing::QuadraticInOut);
    }
}

impl Transition for EaseInOutTransition {
    fn ease_in(&self, time: Instant, duration: Duration) -> AnimatedSlideProperties {
        let mut properties = AnimatedSlideProperties::default();
        properties.set_global_opacity_no_ease(0.0);
        properties.ease_global_opacity(
            1.0,
            time + duration / 2,
            duration / 2,
            Easing::QuadraticInOut,
        );
        properties
    }

    fn ease_out(
        &self,
        time: Instant,
        duration: Duration,
        properties: &mut AnimatedSlideProperties,
    ) {
        properties.ease_global_opacity(
            0.0,
            time ,
            duration / 2,
            Easing::QuadraticInOut,
        );
    }
}
