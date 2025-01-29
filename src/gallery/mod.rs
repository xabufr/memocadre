use anyhow::Result;

mod immich;

pub use self::immich::ImmichGallery;

pub trait Gallery {
    fn get_next_image(&mut self) -> Result<ImageWithDetails>;
}

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
