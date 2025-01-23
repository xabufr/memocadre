use image::ImageReader;
use log::debug;
use std::{io::Cursor, time::Instant};

mod immich_client;

use immich_client::{AssetResponse, AssetType, ImmichClient, SearchRandomRequest};

use crate::configuration::{Conf, ImmichPerson, ImmichSource};

pub trait Gallery {
    fn get_next_image(&mut self) -> ImageWithDetails;
}

pub struct ImageWithDetails {
    pub image: image::DynamicImage,
    pub city: Option<String>,
    pub date_time: Option<String>,
    pub people: Vec<Person>,
}

pub struct Person {
    pub name: Option<String>,
    pub face: Option<BoxInImage>,
}

pub struct BoxInImage {
    pub height: u32,
    pub width: u32,
    pub box_x_start: u32,
    pub box_y_start: u32,
    pub box_x_end: u32,
    pub box_y_end: u32,
}

pub struct ImmichGallery {
    client: ImmichClient,
    next_assets: Vec<AssetResponse>,
    search: SearchRandomRequest,
}

impl ImmichGallery {
    pub fn new(source: &ImmichSource) -> Self {
        let client = ImmichClient::new(&source.url, &source.api_key);
        let search = Self::build_search_query(&client, source);
        Self {
            client,
            search,
            next_assets: vec![],
        }
    }

    fn build_search_query(client: &ImmichClient, source: &ImmichSource) -> SearchRandomRequest {
        let person_ids = source.search.as_ref().and_then(|search| {
            search.persons.as_ref().map(|persons| {
                persons
                    .iter()
                    .flat_map(|p| match p {
                        // FIXME handle non-existing
                        ImmichPerson::Id(id) => vec![id.to_owned()],
                        ImmichPerson::Name(name) => client
                            .search_person(name)
                            .into_iter()
                            .map(|p| p.id)
                            .collect(),
                    })
                    .collect::<Vec<_>>()
            })
        });
        let search = SearchRandomRequest {
            person_ids,
            ..Default::default()
        };
        println!("{search:#?}");
        return search;
    }
}

impl Gallery for ImmichGallery {
    fn get_next_image(&mut self) -> ImageWithDetails {
        let asset = self.get_next_asset();
        let start = Instant::now();
        let img_data = self.client.view_assets(&asset.id);
        let image = ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        debug!("Asset downloaded and decoded in {:?}", start.elapsed());
        return ImageWithDetails {
            image,
            city: asset.exif_info.as_ref().and_then(|i| i.city.clone()),
            date_time: Some(asset.local_date_time.clone()),
            people: Vec::new(),
        };
    }
}
impl ImmichGallery {
    fn get_next_asset(&mut self) -> AssetResponse {
        return if let Some(next) = self.next_assets.pop() {
            next
        } else {
            self.next_assets = self.client.search_random(SearchRandomRequest {
                r#type: Some(AssetType::IMAGE),
                with_exif: Some(true),
                with_people: Some(true),
                ..self.search.clone()
            });
            self.next_assets.pop().unwrap()
        };
    }
}
