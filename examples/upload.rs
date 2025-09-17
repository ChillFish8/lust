use anyhow::Result;
use serde_json::Value;

use std::{fs::File, io::Read};

#[tokio::main]
async fn main() -> Result<()> {
    let mut file = File::open("./examples/example.jpeg")?;

    let mut image = Vec::new();
    file.read_to_end(&mut image)?;

    let client = reqwest::Client::new();

    let res = client
        .post("http://127.0.0.1:8000/v1/images/user-profiles")
        .query(&[("format", "jpeg")])
        .header("content-type", "application/octet-stream")
        .header("content-length", image.len())
        .body(image)
        .send()
        .await?
        .json::<Value>()
        .await?;

    println!("Response: {:?}", res);

    Ok(())
}
