mod render;

use serde::Deserialize;

use image::ImageReader;
use std::io::Cursor;
const KEY: &'static str = "***REMOVED***";
const BASE: &'static str = "***REMOVED***";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WithId {
    id: String,
    original_file_name: String,
    #[serde(rename = "type")]
    type_: String,
}
fn main() {
    let client = reqwest::blocking::Client::new();

    let mut res: Vec<WithId> = client
        .get(format!("{}/api/assets/random", BASE))
        .header("x-api-key", format!("{}", KEY))
        .send()
        .unwrap()
        .json()
        .unwrap();
    println!("Found randoms");

    let i = res.pop().unwrap();
    let data = client
        .get(format!(
            "{}/api/assets/{}/thumbnail?size=preview",
            BASE, i.id
        ))
        .header("x-api-key", KEY)
        .send()
        .unwrap()
        .bytes()
        .unwrap();
    println!(
        "Downloaded random preview: {} ({})",
        i.original_file_name, i.type_
    );
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    println!("{}x{}", img.width(), img.height());

    render::start(img);
}
