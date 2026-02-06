// DOM manipulation example: Query and interact with DOM
use cdpkit::{dom, page, target, Command, CDP};
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

    // Enable domains
    page::Enable::new().send(&cdp, Some(&session)).await?;
    dom::Enable::new().send(&cdp, Some(&session)).await?;

    // Navigate and wait for load
    let mut events = page::LoadEventFired::subscribe(&cdp);
    println!("Navigating to https://example.com");
    page::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;
    events.next().await;
    println!("Page loaded");

    // Get document
    let doc = dom::GetDocument::new().send(&cdp, Some(&session)).await?;
    println!("Document node ID: {}", doc.root.node_id);

    // Query selector for h1
    let result = dom::QuerySelector::new(doc.root.node_id, "h1")
        .send(&cdp, Some(&session))
        .await?;

    if result.node_id > 0 {
        println!("Found h1 element with node ID: {}", result.node_id);
    }

    // Query all paragraphs
    let result = dom::QuerySelectorAll::new(doc.root.node_id, "p")
        .send(&cdp, Some(&session))
        .await?;
    println!("Found {} paragraph elements", result.node_ids.len());

    Ok(())
}
