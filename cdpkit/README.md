# cdpkit

**[English](../README.md)** | [中文文档](../docs/README.zh.md)

Type-safe Rust [Chrome DevTools Protocol (CDP)](https://chromedevtools.github.io/devtools-protocol/) client with async/await support.

## Features

- 🔒 **Type-safe** - All CDP commands and events are strongly typed with compile-time validation
- 🚀 **Async-first** - Built on tokio with full async/await support
- 📡 **Stream-based events** - Handle CDP events using Rust streams with multiplexing and filtering
- 🎯 **Pure protocol client** - Direct CDP access without abstraction layers, full control
- 🔄 **Auto-generated bindings** - Generated from official CDP specification, always up-to-date
- 🪶 **Lightweight** - Minimal dependencies, focused on protocol communication
- 🔌 **Flexible connection** - Connect to running browser instances without process management
- 🧩 **Dynamic commands** - Send arbitrary CDP commands by name when typed bindings aren't needed

## Prerequisites

Start Chrome with remote debugging enabled:

```bash
# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile

# Linux
google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile

# Windows
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222 --user-data-dir=%TEMP%\cdp-profile

# Headless mode (no UI)
chrome --headless --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile
```

> **Important:** If Chrome is already running, `--remote-debugging-port` will be ignored (Chrome merges into the existing instance). Use `--user-data-dir` to force a separate instance, or close all Chrome windows/processes first.

## Quick Start

```rust
use cdpkit::{CDP, page, target};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Chrome
    let cdp = CDP::connect("localhost:9222").await?;

    // Create a new page (browser-level command → pass &cdp)
    let result = target::methods::CreateTarget::new("https://example.com")
        .send(&cdp)
        .await?;

    // Attach to the page
    let attach = target::methods::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp)
        .await?;

    // Create a session for page-level commands
    let session = cdp.session(attach.session_id);

    // Enable page domain
    page::methods::Enable::new().send(&session).await?;

    // Subscribe BEFORE triggering the action that produces events
    let mut events = page::events::LoadEventFired::subscribe(&session);

    page::methods::Navigate::new("https://rust-lang.org")
        .send(&session)
        .await?;

    if let Some(event) = events.next().await {
        println!("Page loaded at {}", event.timestamp);
    }

    Ok(())
}
```

## Installation

```toml
[dependencies]
cdpkit = "0.3"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
futures = "0.3"
```

## Documentation

- **[Examples](examples/)** - Working code examples
- **[API Reference](https://docs.rs/cdpkit)** - Full API documentation

## Why cdpkit?

### Direct Protocol Access

```rust
// Send CDP commands with all parameters
let session = cdp.session(session_id);
page::methods::Navigate::new("https://example.com")
    .with_referrer("https://google.com")
    .with_transition_type(page::types::TransitionType::Link)
    .send(&session)
    .await?;

// Send arbitrary commands dynamically
let result = session.send_raw(
    "Page.navigate",
    serde_json::json!({"url": "https://example.com"}),
).await?;
println!("Frame ID: {}", result["frameId"]);
```

### Powerful Event Handling

```rust
use futures::StreamExt;

// Subscribe to typed events (session-filtered)
let mut load_events = page::events::LoadEventFired::subscribe(&session);
let mut nav_events = page::events::FrameNavigated::subscribe(&session);

// Handle multiple event streams
loop {
    tokio::select! {
        Some(event) = load_events.next() => {
            println!("Page loaded at {}", event.timestamp);
        }
        Some(event) = nav_events.next() => {
            println!("Navigated to {:?}", event.frame.url);
        }
        else => break,
    }
}
```

### Compile-Time Type Safety

```rust
// ✅ Type-checked: required parameters
let cmd = page::methods::Navigate::new("https://example.com");

// ✅ Type-checked: return values
let result = cmd.send(&session).await?;
let frame_id: String = result.frame_id;  // Type is known

// ❌ Compile error: missing parameter
let cmd = page::methods::Navigate::new();  // Error: missing url

// ❌ Compile error: type mismatch
let cmd = page::methods::Navigate::new(123);  // Error: expected String
```

### Use Cases

cdpkit is ideal for:

- **Automated testing** - Precise control over browser behavior
- **Web scraping** - Monitor network requests and responses
- **Performance analysis** - Access detailed performance metrics
- **Debugging tools** - Build custom developer tools
- **Browser extensions** - Low-level CDP access

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## References
- [cdp-use](https://github.com/browser-use/cdp-use)
- [chromiumoxide](https://github.com/mattsse/chromiumoxide)
