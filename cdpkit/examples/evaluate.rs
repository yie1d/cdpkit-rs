// JavaScript evaluation example: Execute JavaScript and get results
use cdpkit::{page, runtime, target, CDP};
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

    // Enable domains
    page::methods::Enable::new()
        .send(&cdp, Some(&session))
        .await?;
    runtime::methods::Enable::new()
        .send(&cdp, Some(&session))
        .await?;

    // Navigate and wait for load
    let mut events = page::events::LoadEventFired::subscribe(&cdp);
    page::methods::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;
    events.next().await;
    println!("Page loaded");

    // Execute JavaScript to get page title
    let result = runtime::methods::Evaluate::new("document.title")
        .with_return_by_value(true)
        .send(&cdp, Some(&session))
        .await?;

    if let Some(value) = result.result.value {
        println!("Page title: {}", value);
    }

    // Execute JavaScript to get page URL
    let result = runtime::methods::Evaluate::new("window.location.href")
        .with_return_by_value(true)
        .send(&cdp, Some(&session))
        .await?;

    if let Some(value) = result.result.value {
        println!("Page URL: {}", value);
    }

    // Execute JavaScript to count elements
    let result = runtime::methods::Evaluate::new("document.querySelectorAll('*').length")
        .with_return_by_value(true)
        .send(&cdp, Some(&session))
        .await?;

    if let Some(value) = result.result.value {
        println!("Total elements: {}", value);
    }

    Ok(())
}
