use anyhow::Result;

use crate::graphics::{epaint_display::TextContainer, Drawable, Graphics, Sprite};

pub struct Slide {
    sprites: Vec<Sprite>,
    text: Option<TextContainer>,
}

impl Slide {
    pub fn new(sprites: Vec<Sprite>, text: Option<TextContainer>) -> Self {
        Self { sprites, text }
    }

    pub fn set_opacity(&mut self, alpha: f32) {
        for sprite in self.sprites.iter_mut() {
            sprite.opacity = alpha;
        }
        self.text.as_mut().map(|text| text.set_opacity(alpha));
    }
}

impl Drawable for Slide {
    fn draw(&self, graphics: &Graphics) -> Result<()> {
        for sprite in self.sprites.iter() {
            graphics.draw(sprite)?;
        }
        if let Some(text) = &self.text {
            graphics.draw(text)?;
        }
        Ok(())
    }
}
