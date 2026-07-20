use std::path::Path;
use thiserror::Error;

const PROTOCOL_REVISION: &str = "f8bae521a2574e3e414b4268d2b9be2b2a633ecf";
const PROTOCOL_BASE_URL: &str =
    "https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol";

fn protocol_url(file: &str) -> String {
    format!("{PROTOCOL_BASE_URL}/{PROTOCOL_REVISION}/json/{file}")
}

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid protocol JSON: {0}")]
    Json(#[from] serde_json::Error),
}

async fn download_protocol(client: &reqwest::Client, file: &str) -> Result<String, FetchError> {
    let body = client
        .get(protocol_url(file))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    serde_json::from_str::<crate::parser::Protocol>(&body)?;
    Ok(body)
}

pub async fn fetch_protocols(output_dir: &Path) -> Result<(), FetchError> {
    std::fs::create_dir_all(output_dir)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    println!("Fetching browser_protocol.json...");
    let browser = download_protocol(&client, "browser_protocol.json").await?;

    println!("Fetching js_protocol.json...");
    let js = download_protocol(&client, "js_protocol.json").await?;

    // Do not replace either snapshot until both responses are valid protocols.
    std::fs::write(output_dir.join("browser_protocol.json"), browser)?;
    std::fs::write(output_dir.join("js_protocol.json"), js)?;

    println!("Protocols downloaded successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_downloads_use_an_immutable_upstream_revision() {
        assert_eq!(PROTOCOL_REVISION.len(), 40);
        assert!(
            PROTOCOL_REVISION
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
            "protocol revision must be a full commit SHA"
        );

        for url in [
            protocol_url("browser_protocol.json"),
            protocol_url("js_protocol.json"),
        ] {
            assert!(url.starts_with(&format!("{PROTOCOL_BASE_URL}/{PROTOCOL_REVISION}/json/")));
        }
    }
}
