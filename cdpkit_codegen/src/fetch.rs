use std::path::Path;
use thiserror::Error;

const BROWSER_PROTOCOL_URL: &str =
    "https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/browser_protocol.json";
const JS_PROTOCOL_URL: &str =
    "https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/js_protocol.json";

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn fetch_protocols(output_dir: &Path) -> Result<(), FetchError> {
    std::fs::create_dir_all(output_dir)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    println!("Fetching browser_protocol.json...");
    let browser = client
        .get(BROWSER_PROTOCOL_URL)
        .send()
        .await?
        .text()
        .await?;
    std::fs::write(output_dir.join("browser_protocol.json"), browser)?;

    println!("Fetching js_protocol.json...");
    let js = client.get(JS_PROTOCOL_URL).send().await?.text().await?;
    std::fs::write(output_dir.join("js_protocol.json"), js)?;

    println!("Protocols downloaded successfully!");
    Ok(())
}
