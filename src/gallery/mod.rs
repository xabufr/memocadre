use anyhow::{bail, Context, Result};
use itertools::Itertools;

use log::error;
mod immich;

use crate::configuration::Source;

pub trait Gallery {
    fn get_next_image(&mut self) -> Result<ImageWithDetails>;
}

trait GalleryProvider: Gallery {}

pub struct ImageWithDetails {
    pub image: image::DynamicImage,
    pub city: Option<String>,
    pub date_time: Option<String>,
    #[allow(dead_code)]
    pub people: Vec<Person>,
}

#[allow(dead_code)]
pub struct Person {
    pub name: Option<String>,
    pub face: Option<BoxInImage>,
}

#[allow(dead_code)]
pub struct BoxInImage {
    pub height: u32,
    pub width: u32,
    pub box_x_start: u32,
    pub box_y_start: u32,
    pub box_x_end: u32,
    pub box_y_end: u32,
}

struct GalleryImpl {
    galleries: Vec<Box<dyn GalleryProvider>>,
    next: usize,
}

pub fn build_sources(sources: &[Source]) -> Result<Box<dyn Gallery>> {
    let galleries = sources
        .iter()
        .enumerate()
        .map(|(id, source)| match source {
            Source::Immich(immich_source) => immich::build_immich_providers(immich_source)
                .context(format!("Cannot build source {id}")),
        })
        .flatten_ok()
        .try_collect()?;
    Ok(Box::new(GalleryImpl { galleries, next: 0 }))
}

impl Gallery for GalleryImpl {
    fn get_next_image(&mut self) -> Result<ImageWithDetails> {
        for _ in 0..self.galleries.len() {
            let res = self.galleries[self.next].get_next_image();
            self.next = (self.next + 1) % self.galleries.len();
            match res {
                Ok(res) => return Ok(res),
                Err(error) => error!("Cannot get next image: {:?}", error),
            }
        }
        bail!("All sources have failed")
    }
}
