// Event handling example: Listen to multiple CDP events
use cdpkit::{page, target, Command, CDP};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::var("CDP_HOST").unwrap_or_else(|_| "localhost:9222".to_string());
    let cdp = CDP::connect(&host).await?;
    println!("Connected to Chrome");

    // Create and attach to page
    let result = target::CreateTarget::new("about:blank")
        .send(&cdp, None)
        .await?;
    let attach = target::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp, None)
        .await?;
    let session = attach.session_id;

    // Enable page domain
    page::Enable::new().send(&cdp, Some(&session)).await?;

    // Subscribe to page events
    let mut load_events = page::LoadEventFired::subscribe(&cdp);
    let mut nav_events = page::FrameNavigated::subscribe(&cdp);

    // Navigate
    println!("Navigating to https://example.com");
    page::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;

    // Listen to events for 5 seconds
    println!("Listening to events...");
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = load_events.next() => {
                println!("Page loaded at: {}", event.timestamp);
            }
            Some(event) = nav_events.next() => {
                println!("Frame navigated: {:?}", event.frame.url);
            }
            _ = &mut timeout => {
                println!("Timeout reached");
                break;
            }
        }
    }

    Ok(())
}
