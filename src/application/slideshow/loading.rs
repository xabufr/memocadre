use std::time::Instant;

use anyhow::Result;
use epaint::{CircleShape, Color32};
use vek::Vec2;

use crate::{
    configuration::LoadingCircleOptions,
    graphics::{Drawable, Graphics, ShapeContainer},
};

const CIRCLE_ELEMENTS: u8 = 12;

pub struct LoadingSlide {
    circles: [ShapeContainer; CIRCLE_ELEMENTS as usize - 1],
    positions: [Vec2<f32>; CIRCLE_ELEMENTS as usize],
    last_time: Instant,
    velocity: u16,
}

impl LoadingSlide {
    pub fn create(graphics: &mut Graphics, config: &LoadingCircleOptions) -> Result<Self> {
        let circle_radius = graphics.get_dimensions().reduce_min() as f32 / 10.0;
        let circle_size = circle_radius * 0.2;

        let circles = array_init::try_array_init(|i| {
            let gray = 255.0 / CIRCLE_ELEMENTS as f32 * (i + 1) as f32;
            let shape =
                CircleShape::filled((0., 0.).into(), circle_size, Color32::from_gray(gray as u8));
            graphics.create_shape(epaint::Shape::Circle(shape), None)
        })?;

        let positions = array_init::array_init(|i| {
            let angle = 2.0 * std::f32::consts::PI / CIRCLE_ELEMENTS as f32 * i as f32;
            let x = angle.cos() * circle_radius;
            let y = angle.sin() * circle_radius;
            Vec2::new(x, y)
        });

        Ok(Self {
            circles,
            positions,
            last_time: Instant::now(),
            velocity: (1000. / config.velocity) as u16 / CIRCLE_ELEMENTS as u16,
        })
    }

    pub fn update(&mut self, graphics: &Graphics, time: Instant) {
        let p = time.duration_since(self.last_time).as_millis() / self.velocity as u128;
        let p = (p % CIRCLE_ELEMENTS as u128) as u8;

        let center: Vec2<f32> = (graphics.get_dimensions().as_() / 2.0).into();
        for (i, circle) in self.circles.iter_mut().enumerate() {
            let position = self.positions[((i as u8 + p) % CIRCLE_ELEMENTS) as usize];
            circle.set_position(position + center);
        }
    }
}

impl Drawable for LoadingSlide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        for circle in self.circles.iter() {
            circle.draw(graphics)?;
        }
        Ok(())
    }
}
