use bytes::Bytes;
use image::ImageReader;
use log::debug;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{io::Cursor, time::Instant};

mod immich_client;

use immich_client::{AssetType, ImmichClient, SearchRandomRequest};

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
            let mut next_batch = self.client.search_random(SearchRandomRequest {
                r#type: Some(AssetType::IMAGE),
                ..Default::default()
            });
            self.next_ids = next_batch.into_iter().map(|asset| asset.id).collect();
            debug!("Found a next batch of {} from Immich", self.next_ids.len());
            self.next_ids.pop().unwrap()
        };
        let start = Instant::now();
        let img_data = self.client.view_assets(next);
        let image = ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        debug!("Asset downloaded and decoded in {:?}", start.elapsed());
        return image;
    }
}
