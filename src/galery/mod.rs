use serde::Deserialize;
use std::io::Cursor;

use image::ImageReader;

pub trait Galery {
    fn get_next_image(&mut self) -> image::DynamicImage;
}

pub struct ImmichGalery {
    base_url: String,
    api_key: String,
}

impl ImmichGalery {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            base_url: base_url.as_ref().into(),
            api_key: api_key.as_ref().into(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WithId {
    id: String,
    original_file_name: String,
    #[serde(rename = "type")]
    type_: String,
}

impl Galery for ImmichGalery {
    fn get_next_image(&mut self) -> image::DynamicImage {
        let next = minreq::get(format!("{}/api/assets/random", self.base_url))
            .with_header("x-api-key", &self.api_key)
            .send()
            .unwrap();
        let mut next = next.json::<Vec<WithId>>().unwrap();
        let img = next.pop().unwrap();
        let img_data = minreq::get(format!(
            "{}/api/assets/{}/thumbnail?size=preview",
            self.base_url, img.id
        ))
        .with_header("x-api-key", &self.api_key)
        .send()
        .unwrap()
        .into_bytes();
        ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
    }
}
