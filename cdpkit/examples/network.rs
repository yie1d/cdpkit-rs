// Network monitoring example: Monitor network requests and responses
use cdpkit::{network, page, target, Method, CDP};
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
    network::methods::Enable::new()
        .send(&cdp, Some(&session))
        .await?;

    // Subscribe to network events
    let mut request_events = network::events::RequestWillBeSent::subscribe(&cdp);
    let mut response_events = network::events::ResponseReceived::subscribe(&cdp);
    let mut load_events = page::events::LoadEventFired::subscribe(&cdp);

    // Navigate
    println!("Navigating to https://example.com");
    page::methods::Navigate::new("https://example.com")
        .send(&cdp, Some(&session))
        .await?;

    // Monitor network activity
    let mut request_count = 0;
    let mut response_count = 0;

    loop {
        tokio::select! {
            Some(event) = request_events.next() => {
                request_count += 1;
                println!("[Request #{}] {} {}",
                    request_count,
                    event.request.method,
                    event.request.url
                );
            }
            Some(event) = response_events.next() => {
                response_count += 1;
                println!("[Response #{}] {} - Status: {}",
                    response_count,
                    event.response.url,
                    event.response.status
                );
            }
            Some(_) = load_events.next() => {
                println!("\nPage loaded!");
                println!("Total requests: {}", request_count);
                println!("Total responses: {}", response_count);
                break;
            }
        }
    }

    Ok(())
}
