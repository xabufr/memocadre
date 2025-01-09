use std::io::Cursor;

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use image::ImageReader;

pub trait Galery {
    fn get_next_image(&mut self) -> image::DynamicImage;
}

pub struct ImmichGalery {
    client: ImmichClient,
}

impl ImmichGalery {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            client: ImmichClient::new(base_url, api_key),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AssetResponse {
    pub id: String,
    // pub original_file_name: String,
    // pub r#type: AssetType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetType {
    IMAGE,
    VIDEO,
    AUDIO,
    OTHER,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchRandomRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    library_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    person_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_people: Option<bool>,
}

impl Galery for ImmichGalery {
    fn get_next_image(&mut self) -> image::DynamicImage {
        let mut next = self.client.search_random(SearchRandomRequest::default());
        let img = next.pop().unwrap();
        let img_data = self.client.view_assets(img.id);
        ImageReader::new(Cursor::new(&img_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
    }
}

struct ImmichClient {
    base_url: String,
    api_key: String,
    client: reqwest::blocking::Client,
}

impl ImmichClient {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            base_url: base_url.as_ref().into(),
            api_key: api_key.as_ref().into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn search_random(&self, query: SearchRandomRequest) -> Vec<AssetResponse> {
        self.post("search/random")
            .json(&query)
            .header("Accept", "application/json")
            .send()
            .unwrap()
            .json()
            .unwrap()
    }

    pub fn view_assets(&self, id: String) -> Bytes {
        self.get(format!("assets/{}/thumbnail?size=preview", id))
            .send()
            .unwrap()
            .bytes()
            .unwrap()
    }

    fn post(&self, path: impl AsRef<str>) -> reqwest::blocking::RequestBuilder {
        self.request(Method::POST, path)
    }

    fn get(&self, path: impl AsRef<str>) -> reqwest::blocking::RequestBuilder {
        self.request(Method::GET, path)
    }

    fn request(&self, method: Method, path: impl AsRef<str>) -> reqwest::blocking::RequestBuilder {
        let url = format!("{}/api/{}", self.base_url, path.as_ref());
        self.client
            .request(method, url)
            .header("x-api-key", &self.api_key)
    }
}
