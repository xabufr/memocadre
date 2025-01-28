use anyhow::{Context, Result};
use image::ImageReader;
use itertools::Itertools;
use log::debug;
use std::{io::Cursor, time::Instant};

mod immich_client;

use immich_client::{AssetResponse, AssetType, ImmichClient, SearchRandomRequest};

use crate::configuration::{ImmichPerson, ImmichSource};

pub trait Gallery {
    fn get_next_image(&mut self) -> Result<ImageWithDetails>;
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
    pub fn new(source: &ImmichSource) -> Result<Self> {
        let client = ImmichClient::new(&source.url, &source.api_key);
        let search =
            Self::build_search_query(&client, source).context("Cannot build search query")?;
        Ok(Self {
            client,
            search,
            next_assets: vec![],
        })
    }

    fn build_search_query(
        client: &ImmichClient,
        source: &ImmichSource,
    ) -> Result<SearchRandomRequest> {
        let person_ids = source
            .search
            .as_ref()
            .and_then(|search| {
                search.persons.as_ref().map(|persons| -> Result<_> {
                    persons
                        .iter()
                        .map(|p| -> Result<_> {
                            Ok(match p {
                                // FIXME handle non-existing
                                ImmichPerson::Id(id) => vec![id.to_owned()],
                                ImmichPerson::Name(name) => client
                                    .search_person(name)?
                                    .into_iter()
                                    .map(|p| p.id)
                                    .collect(),
                            })
                        })
                        .flatten_ok()
                        .collect::<Result<Vec<_>>>()
                })
            })
            .transpose()?;
        let search = SearchRandomRequest {
            person_ids,
            ..Default::default()
        };
        println!("{search:#?}");
        return Ok(search);
    }
}

impl Gallery for ImmichGallery {
    fn get_next_image(&mut self) -> Result<ImageWithDetails> {
        let asset = self.get_next_asset()?;
        let start = Instant::now();
        let img_data = self
            .client
            .view_assets(&asset.id)
            .context("Cannot fetch image data")?;
        let image = ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .context("Cannot guess image format")?
            .decode()
            .context("Cannot decode image")?;
        debug!("Asset downloaded and decoded in {:?}", start.elapsed());
        return Ok(ImageWithDetails {
            image,
            city: asset.exif_info.as_ref().and_then(|i| i.city.clone()),
            date_time: Some(asset.local_date_time.clone()),
            people: Vec::new(),
        });
    }
}
impl ImmichGallery {
    fn get_next_asset(&mut self) -> Result<AssetResponse> {
        return if let Some(next) = self.next_assets.pop() {
            Ok(next)
        } else {
            self.next_assets = self
                .client
                .search_random(SearchRandomRequest {
                    r#type: Some(AssetType::IMAGE),
                    with_exif: Some(true),
                    with_people: Some(true),
                    ..self.search.clone()
                })
                .context("Error while search next assets batch")?;
            self.next_assets
                .pop()
                .context("Should have at least one asset")
        };
    }
}
