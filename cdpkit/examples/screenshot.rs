// Screenshot example: Capture page screenshot
use cdpkit::{page, target, Method, CDP};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::var("CDP_HOST").unwrap_or_else(|_| "localhost:9222".to_string());
    let cdp = CDP::connect(&host).await?;
    println!("Connected to Chrome");

    // Create and attach to page
    let result = target::methods::CreateTarget::new("about:blank")
        .send(&cdp, None)
        .await?;
    let attach = target::methods::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp, None)
        .await?;
    let session = attach.session_id;

    // Enable page domain
    page::methods::Enable::new()
        .send(&cdp, Some(&session))
        .await?;

    // Navigate and wait for load
    let mut events = page::events::LoadEventFired::subscribe(&cdp);
    println!("Navigating to https://example.com");
    page::methods::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;
    events.next().await;
    println!("Page loaded");

    // Wait a bit for rendering
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Capture screenshot
    println!("Capturing screenshot...");
    let result = page::methods::CaptureScreenshot::new()
        .with_format("png")
        .send(&cdp, Some(&session))
        .await?;

    // Decode and save
    let image_data = base64::decode(&result.data)?;
    std::fs::write("screenshot.png", image_data)?;
    println!(
        "Screenshot saved to screenshot.png ({} bytes)",
        result.data.len()
    );

    Ok(())
}
