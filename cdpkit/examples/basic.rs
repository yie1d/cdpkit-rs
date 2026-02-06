// Basic example: Connect to Chrome and navigate to a page
use cdpkit::{page, target, Command, CDP};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Chrome (auto-discovers WebSocket URL)
    let host = std::env::var("CDP_HOST").unwrap_or_else(|_| "localhost:9222".to_string());
    let cdp = CDP::connect(&host).await?;
    println!("Connected to Chrome");

    // Create a new page
    let result = target::CreateTarget::new("about:blank")
        .send(&cdp, None)
        .await?;
    println!("Created target: {}", result.target_id);

    // Attach to the page
    let attach = target::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp, None)
        .await?;
    let session = attach.session_id;
    println!("Attached to session: {}", session);

    // Enable page domain
    page::Enable::new().send(&cdp, Some(&session)).await?;

    // Subscribe to load events
    let mut events = page::LoadEventFired::subscribe(&cdp);

    // Navigate to a page
    println!("Navigating to https://example.com");
    page::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;

    // Wait for page load
    if let Some(event) = events.next().await {
        println!("Page loaded at timestamp: {}", event.timestamp);
    }

    Ok(())
}
