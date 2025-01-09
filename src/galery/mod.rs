use std::io::Cursor;

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use image::ImageReader;

mod immich_client;

use immich_client::{ImmichClient, SearchRandomRequest};

pub trait Galery {
    fn get_next_image(&mut self) -> image::DynamicImage;
}

pub struct ImmichGalery {
    client: ImmichClient,
    next_ids: Vec<String>,
}

impl ImmichGalery {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            client: ImmichClient::new(base_url, api_key),
            next_ids: vec![],
        }
    }
}

impl Galery for ImmichGalery {
    fn get_next_image(&mut self) -> image::DynamicImage {
        let next = if let Some(id) = self.next_ids.pop() {
            id
        } else {
            let mut next_batch = self.client.search_random(SearchRandomRequest::default());
            self.next_ids = next_batch.into_iter().map(|asset| asset.id).collect();
            self.next_ids.pop().unwrap()
        };
        let img_data = self.client.view_assets(next);
        ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
    }
}
