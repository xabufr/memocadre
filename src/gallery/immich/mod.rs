use std::{io::Cursor, ops::Deref, rc::Rc, time::Instant};

use anyhow::{Context, Result};
use client::SmartSearchRequest;
use image::ImageReader;
use itertools::Itertools;
use log::debug;

use self::client::{AssetResponse, AssetType, ImmichClient, SearchRandomRequest};
use super::{Gallery, GalleryProvider};
use crate::{
    configuration::{ImmichPerson, ImmichSearchQuery, ImmichSource, ImmichSpec},
    gallery::ImageWithDetails,
};

mod client;

struct ImmichGalleryProvider {
    client: Rc<ImmichClient>,
    search: ImmichRequest,
    next_assets: Vec<AssetResponse>,
}

#[derive(Debug)]
enum ImmichRequest {
    RandomSearch(SearchRandomRequest),
    SmartSearch(SmartSearchRequest),
    PrivateAlbum { id: String },
    MemoryLane,
}

impl ImmichRequest {
    fn load_next(&self, client: &ImmichClient) -> Result<Vec<AssetResponse>> {
        match self {
            ImmichRequest::RandomSearch(search_random_request) => Ok(client
                .search_random(SearchRandomRequest {
                    r#type: Some(AssetType::IMAGE),
                    with_exif: Some(true),
                    ..search_random_request.clone()
                })
                .context("Error while search next assets batch")?),
            ImmichRequest::SmartSearch(request) => Ok(client
                .smart_search(SmartSearchRequest {
                    r#type: Some(AssetType::IMAGE),
                    with_exif: Some(true),
                    ..request.clone()
                })
                .context("Error while smart searching next assets batch")?
                .assets
                .items),
            ImmichRequest::PrivateAlbum { id } => Ok(client
                .get_album(id)
                .context("Cannot get album for next batch")?
                .assets),
            ImmichRequest::MemoryLane => Ok(client
                .get_memory_lane(29, 1)?
                .into_iter()
                .flat_map(|l| l.assets)
                .collect()),
        }
    }
}

impl Gallery for ImmichGalleryProvider {
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
            date: Some(asset.file_created_at),
            people: Vec::new(),
        });
    }
}
impl GalleryProvider for ImmichGalleryProvider {}

impl ImmichGalleryProvider {
    fn new(client: &Rc<ImmichClient>, search: &ImmichSpec) -> Result<Self> {
        let immich_request = match search {
            ImmichSpec::RandomSearch(immich_search_query) => {
                let req = Self::build_random_search(client.deref(), immich_search_query)
                    .context("While building search request")?;
                ImmichRequest::RandomSearch(req)
            }
            ImmichSpec::SmartSearch(search) => ImmichRequest::SmartSearch(SmartSearchRequest {
                person_ids: Self::get_persons_ids(client.deref(), &search.persons)?,
                city: search.city.clone(),
                query: search.query.clone(),
                ..Default::default()
            }),
            ImmichSpec::PrivateAlbum { id } => ImmichRequest::PrivateAlbum { id: id.clone() },
            ImmichSpec::MemoryLane => ImmichRequest::MemoryLane,
        };
        let immich_request = immich_request;
        let search = immich_request;
        Ok(Self {
            client: client.clone(),
            next_assets: Vec::new(),
            search,
        })
    }

    fn build_random_search(
        client: &ImmichClient,
        search: &ImmichSearchQuery,
    ) -> Result<SearchRandomRequest> {
        let person_ids = Self::get_persons_ids(client, &search.persons)?;
        return Ok(SearchRandomRequest {
            person_ids,
            ..Default::default()
        });
    }

    fn get_persons_ids(
        client: &ImmichClient,
        persons: &Option<Vec<ImmichPerson>>,
    ) -> Result<Option<Vec<String>>> {
        persons
            .as_ref()
            .map(|persons| -> Result<_> {
                persons
                    .iter()
                    .map(|p| -> Result<_> {
                        Ok(match p {
                            // FIXME handle non-existing
                            ImmichPerson::Id(id) => vec![id.to_owned()],
                            ImmichPerson::Name(name) => client
                                .search_person(name)
                                .context("Cannot list persons")?
                                .into_iter()
                                .map(|p| p.id)
                                .collect(),
                        })
                    })
                    .flatten_ok()
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()
    }

    fn get_next_asset(&mut self) -> Result<AssetResponse> {
        let asset = if let Some(next) = self.next_assets.pop() {
            next
        } else {
            self.next_assets = self
                .search
                .load_next(&self.client)
                .context("Error while loading next asset batch")?;
            self.next_assets
                .pop()
                .context("Should have at least one asset")?
        };
        self.client
            .get_asset_details(&asset.id)
            .context("Cannot fetch assets with details")
    }
}

pub fn build_immich_providers(source: &ImmichSource) -> Result<Vec<Box<dyn GalleryProvider>>> {
    source
        .instance
        .iter()
        .chain(source.instances.iter())
        .enumerate()
        .flat_map(|(id, instance)| {
            let client = ImmichClient::new(&instance.url, &instance.api_key);
            let client = Rc::new(client);
            source
                .specs
                .iter()
                .map(move |search| ImmichGalleryProvider::new(&client, search))
                .map(move |p| match p {
                    Ok(p) => Ok(Box::new(p) as Box<dyn GalleryProvider>),
                    Err(err) => Err(err).context(format!("Cannot build for client {id}")),
                })
        })
        .try_collect()
}
