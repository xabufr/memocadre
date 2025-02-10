use std::time::Instant;

use glissade::{Animated as _, Easing, Inertial};
use paste::paste;
use std::time::Duration;

use super::slide::SlideProperties;

type Animated<T> = Inertial<T, Instant>;

macro_rules! animated_properties {
    (
        $struct_name:ident {
           $(
               $field_name:ident: $field_type:ty = $default:expr,
           )*
        }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $struct_name {
            $(
                $field_name: Animated<$field_type>,
            )*
        }

        impl $struct_name {
            #[allow(dead_code)]
            pub fn is_finished(&self, instant: Instant) -> bool {
                $(
                    self.$field_name.is_finished(instant)
                )&&*
            }

            #[allow(dead_code)]
            pub fn to_slide_properties(&self, instant: Instant) -> SlideProperties {
                SlideProperties {
                    $(
                        $field_name: self.$field_name.get(instant),
                    )*
                }
            }

            #[allow(dead_code)]
            pub fn get_target(&self) -> SlideProperties {
                SlideProperties {
                    $(
                        $field_name: self.$field_name.target(),
                    )*
                }
            }

            $(
                paste! {
                    #[allow(dead_code)]
                    pub fn [<ease_ $field_name>](
                        &mut self,
                        target: $field_type,
                        start: Instant,
                        duration: Duration,
                        ease: Easing,
                    ) {
                        let mut field = Animated::new(target);
                        std::mem::swap(&mut self.$field_name, &mut field);
                        field = field.ease_to(target, start, duration, ease);
                        std::mem::swap(&mut self.$field_name, &mut field);
                    }

                    #[allow(dead_code)]
                    pub fn [<then_ease $field_name>](
                        &mut self,
                        target: $field_type,
                        duration: Duration,
                        now: Instant,
                        ease: Easing,
                    ) {
                        let mut field = Animated::new(target);
                        std::mem::swap(&mut self.$field_name, &mut field);
                        let start = field.end_time().unwrap_or(now);
                        field = field.ease_to(target, start, duration, ease);
                        std::mem::swap(&mut self.$field_name, &mut field);
                    }

                    #[allow(dead_code)]
                    pub fn [<set_ $field_name _no_ease>](
                        &mut self,
                        value: $field_type,
                    ) {
                        self.$field_name = Animated::new(value);
                    }

                    #[allow(dead_code)]
                    pub fn [<get_target_ $field_name>](
                        &self
                    ) -> $field_type {
                        self.$field_name.target()
                    }
                }
            )*
        }

        impl From<SlideProperties> for $struct_name {
            fn from(properties: SlideProperties) -> Self {
                $struct_name {
                    $(
                        $field_name: Animated::new(properties.$field_name),
                    )*
                }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                $struct_name {
                    $(
                        $field_name: Animated::new($default),
                    )*
                }
            }
        }
    };
}

animated_properties!(AnimatedSlideProperties {
    global_opacity: f32 = 1.0,
    zoom: f32 = 1.0,
});

#[cfg(test)]
mod test {
    use glissade::Animated;
    use googletest::{
        expect_that, gtest,
        prelude::{eq, is_false, is_true},
    };

    use super::*;

    #[gtest]
    fn test_animated_properties_defaults() {
        let now = Instant::now();
        let properties = AnimatedSlideProperties::default();
        expect_that!(properties.global_opacity.get(now), eq(1.0));
        expect_that!(properties.zoom.get(now), eq(1.0));
        expect_that!(properties.is_finished(now), is_true());
        expect_that!(
            properties.is_finished(now - Duration::from_secs(100)),
            is_true()
        );

        expect_that!(properties.get_target_global_opacity(), eq(1.0));
        expect_that!(properties.get_target_zoom(), eq(1.0));
    }

    #[gtest]
    fn test_animation() {
        let now = Instant::now();
        let mut properties = AnimatedSlideProperties::default();
        properties.ease_global_opacity(0.0, now, Duration::from_secs(1), Easing::Linear);
        expect_that!(properties.global_opacity.get(now), eq(1.0));
        expect_that!(properties.is_finished(now), is_false());
        expect_that!(
            properties.is_finished(now + Duration::from_millis(999)),
            is_false()
        );
        expect_that!(
            properties.global_opacity.get(now + Duration::from_secs(1)),
            eq(0.0)
        );
        expect_that!(
            properties.is_finished(now + Duration::from_secs_f32(1.1)),
            is_true()
        );
        expect_that!(properties.get_target_global_opacity(), eq(0.0));
    }

    #[gtest]
    fn test_set_no_ease() {
        let now = Instant::now();
        let mut properties = AnimatedSlideProperties::default();
        properties.ease_global_opacity(0.0, now, Duration::from_secs(1), Easing::Linear);
        properties.set_global_opacity_no_ease(1.0);
        expect_that!(
            properties
                .global_opacity
                .get(now + Duration::from_millis(500)),
            eq(1.0)
        );
        expect_that!(properties.is_finished(now), is_true());
    }

    #[gtest]
    fn to_slide_properties() {
        let now = Instant::now();
        let mut properties = AnimatedSlideProperties::default();
        properties.ease_global_opacity(0.0, now, Duration::from_secs(1), Easing::Linear);
        properties.ease_zoom(2.0, now, Duration::from_secs(2), Easing::Linear);
        let slide_properties = properties.to_slide_properties(now + Duration::from_secs(1));
        expect_that!(slide_properties.global_opacity, eq(0.0));
        expect_that!(slide_properties.zoom, eq(1.5));
    }

    #[gtest]
    fn to_slide_properties_target() {
        let now = Instant::now();
        let mut properties = AnimatedSlideProperties::default();
        properties.ease_global_opacity(0.0, now, Duration::from_secs(1), Easing::Linear);
        properties.ease_zoom(2.0, now, Duration::from_secs(2), Easing::Linear);
        let slide_properties = properties.get_target();
        expect_that!(slide_properties.global_opacity, eq(0.0));
        expect_that!(slide_properties.zoom, eq(2.0));
    }

    #[gtest]
    fn from_slide_properties() {
        let now = Instant::now();
        let properties = SlideProperties {
            global_opacity: 0.0,
            zoom: 2.0,
        };
        let properties = AnimatedSlideProperties::from(properties);
        expect_that!(properties.is_finished(now), is_true());
        let properties = properties.get_target();
        expect_that!(properties.global_opacity, eq(0.0));
        expect_that!(properties.zoom, eq(2.0));
    }
}
