use std::{
    mem::MaybeUninit,
    time::{Duration, Instant},
};

use crate::graphics::{Drawable, Graphics, ShapeContainer};
use anyhow::Result;
use epaint::{CircleShape, Color32};
use vek::Vec2;

const CIRCLE_ELEMENTS: u8 = 12;

pub struct LoadingSlide {
    circles: [ShapeContainer; CIRCLE_ELEMENTS as usize - 1],
    positions: [Vec2<f32>; CIRCLE_ELEMENTS as usize],
    last_time: Instant,
    velocity: u16,
}

impl LoadingSlide {
    pub fn create(graphics: &mut Graphics, config: &Conf) -> Result<Self> {
        let circle_radius = graphics.get_dimensions().reduce_min() as f32 / 10.0;
        let circle_size = circle_radius * 0.2;

        let mut circles: [MaybeUninit<ShapeContainer>; CIRCLE_ELEMENTS as usize - 1] =
            [const { MaybeUninit::uninit() }; CIRCLE_ELEMENTS as usize - 1];
        for (i, elem) in circles.iter_mut().enumerate() {
            let gray = 255.0 / CIRCLE_ELEMENTS as f32 * (i + 1) as f32;
            let shape =
                CircleShape::filled((0., 0.).into(), circle_size, Color32::from_gray(gray as u8));
            let circle = graphics.create_shape(epaint::Shape::Circle(shape), None)?;
            elem.write(circle);
        }

        let mut positions: [MaybeUninit<Vec2<f32>>; CIRCLE_ELEMENTS as usize] =
            [const { MaybeUninit::uninit() }; CIRCLE_ELEMENTS as usize];
        for (i, elem) in positions.iter_mut().enumerate() {
            let angle = 2.0 * std::f32::consts::PI / CIRCLE_ELEMENTS as f32 * i as f32;
            let x = angle.cos() * circle_radius;
            let y = angle.sin() * circle_radius;
            elem.write(Vec2::new(x, y));
        }

        unsafe {
            Ok(Self {
                circles: std::mem::transmute(circles),
                positions: std::mem::transmute(positions),
                last_time: Instant::now(),
                velocity: 1000 / CIRCLE_ELEMENTS as u16,
            })
        }
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
