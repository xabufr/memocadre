use std::{io::Cursor, time::Instant};

use anyhow::{Context, Result};
use image::ImageReader;
use itertools::Itertools;
use log::debug;

use self::client::{AssetResponse, AssetType, ImmichClient, SearchRandomRequest};
use super::Gallery;
use crate::{
    configuration::{ImmichInstance, ImmichPerson, ImmichSearch, ImmichSearchQuery, ImmichSource},
    gallery::ImageWithDetails,
};

mod client;

pub struct ImmichGallery {
    clients_and_searches: Vec<ClientAndSearch>,
    next_client: usize,
}

struct ClientAndSearch {
    client: ImmichClient,
    searches: Vec<SearchRandomRequest>,
    next_assets: Vec<Vec<AssetResponse>>,
    next_search: usize,
}

impl ClientAndSearch {
    fn new(instance: &ImmichInstance, searches: &Vec<ImmichSearch>) -> Result<Self> {
        let client = ImmichClient::new(&instance.url, &instance.api_key);
        let searches: Vec<_> = searches
            .iter()
            .map(|search| Self::build_search_query(&client, search))
            .try_collect()?;
        let next_assets = searches.iter().map(|_| Vec::new()).collect();
        Ok(Self {
            client,
            searches,
            next_assets,
            next_search: 0,
        })
    }

    fn build_search_query(
        client: &ImmichClient,
        source: &ImmichSearch,
    ) -> Result<SearchRandomRequest> {
        match source {
            ImmichSearch::RandomSearch(immich_search_query) => {
                Self::build_random_search(client, immich_search_query)
            }
        }
    }

    fn build_random_search(
        client: &ImmichClient,
        search: &ImmichSearchQuery,
    ) -> Result<SearchRandomRequest> {
        let person_ids = search
            .persons
            .as_ref()
            .map(|persons| -> Result<_> {
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
            .transpose()?;
        let search = SearchRandomRequest {
            person_ids,
            ..Default::default()
        };
        return Ok(search);
    }

    fn get_next_asset(&mut self) -> Result<AssetResponse> {
        let next = if let Some(next) = self.next_assets[self.next_search].pop() {
            Ok(next)
        } else {
            self.next_assets[self.next_search] = self
                .client
                .search_random(SearchRandomRequest {
                    r#type: Some(AssetType::IMAGE),
                    with_exif: Some(true),
                    with_people: Some(true),
                    ..self.searches[self.next_search].clone()
                })
                .context("Error while search next assets batch")?;
            self.next_assets[self.next_search]
                .pop()
                .context("Should have at least one asset")
        };
        self.next_search = (self.next_search + 1) % self.searches.len();
        return next;
    }

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

impl Gallery for ImmichGallery {
    fn get_next_image(&mut self) -> Result<ImageWithDetails> {
        let res = self.clients_and_searches[self.next_client].get_next_image();
        self.next_client = (self.next_client + 1) % self.clients_and_searches.len();
        return res;
    }
}

impl ImmichGallery {
    pub fn new(source: &ImmichSource) -> Result<Self> {
        let clients_and_searches = source
            .instance
            .iter()
            .chain(source.instances.iter())
            .map(|instance| ClientAndSearch::new(instance, &source.searches))
            .try_collect()?;
        Ok(Self {
            clients_and_searches,
            next_client: 0,
        })
    }
}
