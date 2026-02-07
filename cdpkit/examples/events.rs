// Event handling example: Listen to multiple CDP events
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

    // Subscribe to page events
    let mut load_events = page::events::LoadEventFired::subscribe(&cdp);
    let mut nav_events = page::events::FrameNavigated::subscribe(&cdp);

    // Navigate to first page
    println!("Navigating to https://example.com");
    page::methods::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;

    // Listen to events with a short timeout, then navigate again
    println!("Listening to events...");
    let mut event_count = 0;
    let mut navigated_second = false;
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(event) = load_events.next() => {
                event_count += 1;
                println!("[{}] Page loaded at: {}", event_count, event.timestamp);

                // After first load, navigate to a second page
                if !navigated_second {
                    navigated_second = true;
                    println!("Navigating to https://rust-lang.org");
                    page::methods::Navigate::new("https://rust-lang.org")
                        .send(&cdp, Some(&session))
                        .await?;
                }
            }
            Some(event) = nav_events.next() => {
                event_count += 1;
                println!("[{}] Frame navigated: {:?}", event_count, event.frame.url);
            }
            _ = &mut timeout => {
                println!("Timeout reached after {} events", event_count);
                break;
            }
        }
    }

    Ok(())
}
