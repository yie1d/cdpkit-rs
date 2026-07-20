// Basic example: Connect to Chrome and navigate to a page
use cdpkit::{page, target, CDP};
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Chrome (auto-discovers WebSocket URL)
    // Set CDP_HOST env var to override (e.g., CDP_HOST=localhost:9333)
    let host = std::env::var("CDP_HOST").unwrap_or_else(|_| "localhost:9222".to_string());
    let cdp = CDP::connect(&host).await?;
    println!("Connected to Chrome");

    // Create a new page (browser-level command → pass &cdp)
    let result = target::methods::CreateTarget::new("about:blank")
        .send(&cdp)
        .await?;
    let target_id = result.target_id.clone();
    println!("Created target: {}", target_id);

    // Attach to the page (with_flatten enables event delivery on this connection)
    let attach = target::methods::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp)
        .await?;

    // Create a Session — all subsequent page commands use &session
    let session = cdp.session(attach.session_id);

    // Enable page domain (required before receiving page events)
    page::methods::Enable::new().send(&session).await?;

    // Subscribe to events BEFORE triggering the action that produces them
    let mut events = page::events::LoadEventFired::subscribe(&session);

    // Navigate to a page
    println!("Navigating to https://example.com");
    page::methods::Navigate::new("https://example.com")
        .send(&session)
        .await?;

    // Wait for page load
    if let Some(event) = events.next().await {
        println!("Page loaded at timestamp: {}", event.timestamp);
    }

    // Clean up: close the target
    target::methods::CloseTarget::new(target_id)
        .send(&cdp)
        .await?;

    // Gracefully close the connection
    cdp.close().await;
    // Wait until the background WebSocket loop has completed shutdown.
    tokio::time::timeout(Duration::from_secs(5), cdp.closed()).await?;

    Ok(())
}
