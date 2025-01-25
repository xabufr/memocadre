use log::trace;
use minreq::{Method, Request};
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

impl ImmichClient {
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Self {
        Self {
            base_url: base_url.as_ref().into(),
            api_key: api_key.as_ref().into(),
        }
    }

    pub fn search_random(&self, query: SearchRandomRequest) -> Vec<AssetResponse> {
        self.post("search/random")
            .with_json(&query)
            .unwrap()
            .with_header("Accept", "application/json")
            .send()
            .unwrap()
            .json()
            .unwrap()
    }

    pub fn search_person(&self, name: &str) -> Vec<PersonResponse> {
        self.get(format!("search/person"))
            .with_param("name", name)
            .send()
            .unwrap()
            .json()
            .unwrap()
    }

    pub fn view_assets(&self, id: &str) -> Vec<u8> {
        self.get(format!("assets/{}/thumbnail?size=preview", id))
            .send()
            .unwrap()
            .into_bytes()
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
