use anyhow::{anyhow, Context, Result};
use log::trace;
use minreq::{Method, Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AssetResponse {
    pub id: String,
    pub exif_info: Option<ExifInfo>,
    pub local_date_time: String,
    pub r#type: AssetType,
    pub people: Vec<Person>,
    #[serde(default = "Vec::default")]
    pub unassigned_faces: Vec<Face>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AlbumInfo {
    pub album_name: String,
    pub id: String,
    pub assets: Vec<AssetResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MemoryLaneElement {
    pub years_ago: i32,
    pub assets: Vec<AssetResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Person {
    pub name: String,
    pub faces: Vec<Face>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Face {
    pub id: String,
    pub image_height: i32,
    pub image_width: i32,
    #[serde(rename = "boundingBoxX1")]
    pub bounding_box_x1: i32,
    #[serde(rename = "boundingBoxX2")]
    pub bounding_box_x2: i32,
    #[serde(rename = "boundingBoxY1")]
    pub bounding_box_y1: i32,
    #[serde(rename = "boundingBoxY2")]
    pub bounding_box_y2: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ExifInfo {
    pub city: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum AssetType {
    IMAGE,
    VIDEO,
    AUDIO,
    OTHER,
}

#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchRandomRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssetType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_people: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_exif: Option<bool>,
}

#[derive(Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SmartSearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssetType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_people: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_exif: Option<bool>,
    pub query: String,
}
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SmartSearchResponse {
    pub assets: SmartSearchAssets,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SmartSearchAssets {
    pub items: Vec<AssetResponse>,
}

pub struct ImmichClient {
    base_url: String,
    api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PersonResponse {
    pub id: String,
    pub birth_date: Option<String>,
    pub name: String,
}

// TODO Handle status code error
impl ImmichClient {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            base_url: base_url.as_ref().into(),
            api_key: api_key.as_ref().into(),
        }
    }

    pub fn smart_search(&self, query: SmartSearchRequest) -> Result<SmartSearchResponse> {
        self.handle_error(
            self.post("search/smart")
                .with_json(&query)
                .context("Cannot send SmartSearch query")?
                .with_header("Accept", "application/json")
                .send(),
        )?
        .json()
        .context("Cannot read response")
    }

    pub fn search_random(&self, query: SearchRandomRequest) -> Result<Vec<AssetResponse>> {
        self.handle_error(
            self.post("search/random")
                .with_json(&query)?
                .with_header("Accept", "application/json")
                .send(),
        )?
        .json()
        .context("Cannot read response")
    }

    pub fn get_album(&self, id: &str) -> Result<AlbumInfo> {
        self.handle_error(self.get(format!("albums/{id}")).send())?
            .json()
            .context("Cannot read response")
    }

    pub fn search_person(&self, name: &str) -> Result<Vec<PersonResponse>> {
        self.handle_error(self.get("search/person").with_param("name", name).send())?
            .json()
            .context("Cannot read response")
    }

    pub fn get_memory_lane(&self, day: u8, month: u8) -> Result<Vec<MemoryLaneElement>> {
        self.handle_error(
            self.get("assets/memory-lane")
                .with_param("day", &day.to_string())
                .with_param("month", &month.to_string())
                .send(),
        )?
        .json()
        .context("Cannot read immich response")
    }

    pub fn get_asset_details(&self, id: &str) -> Result<AssetResponse> {
        self.handle_error(self.get(format!("assets/{id}")).send())?
            .json()
            .context("Cannot read response")
    }

    pub fn view_assets(&self, id: &str) -> Result<Vec<u8>> {
        Ok(self
            .handle_error(
                self.get(format!("assets/{id}/thumbnail?size=preview"))
                    .send(),
            )?
            .into_bytes())
    }

    fn handle_error(
        &self,
        response: core::result::Result<Response, minreq::Error>,
    ) -> Result<Response> {
        let response = response.context("Cannot send request")?;
        if response.status_code >= 400 {
            Err(anyhow!(
                "Response error: status code {} ({})",
                response.status_code,
                response.reason_phrase
            ))
        } else {
            Ok(response)
        }
    }

    fn post(&self, path: impl AsRef<str>) -> Request {
        self.request(Method::Post, path)
    }

    fn get(&self, path: impl AsRef<str>) -> Request {
        self.request(Method::Get, path)
    }

    fn request(&self, method: Method, path: impl AsRef<str>) -> Request {
        let url = format!("{}/api/{}", self.base_url, path.as_ref());
        trace!("Requesting Immich with {} {}", method, url);
        Request::new(method, url).with_header("x-api-key", &self.api_key)
    }
}
