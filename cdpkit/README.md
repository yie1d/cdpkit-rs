# cdpkit

**[English](README.md)** | [中文文档](docs/README.zh.md)

Type-safe Rust [Chrome DevTools Protocol (CDP)](https://chromedevtools.github.io/devtools-protocol/) client with async/await support.

## Features

- 🔒 **Type-safe** - All CDP commands and events are strongly typed with compile-time validation
- 🚀 **Async-first** - Built on tokio with full async/await support
- 📡 **Stream-based events** - Handle CDP events using Rust streams with multiplexing and filtering
- 🎯 **Pure protocol client** - Direct CDP access without abstraction layers, full control
- 🔄 **Auto-generated bindings** - Generated from official CDP specification, always up-to-date
- 🪶 **Lightweight** - Minimal dependencies, focused on protocol communication
- 🔌 **Flexible connection** - Connect to running browser instances without process management

## Quick Start

```rust
use cdpkit::{CDP, Method, page, target};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Chrome
    let cdp = CDP::connect("localhost:9222").await?;
    
    // Create a new page
    let result = target::methods::CreateTarget::new("https://example.com")
        .send(&cdp, None)
        .await?;
    
    // Attach to the page
    let attach = target::methods::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp, None)
        .await?;
    
    let session = attach.session_id;
    
    // Navigate and listen to events
    page::methods::Enable::new().send(&cdp, Some(&session)).await?;
    page::methods::Navigate::new("https://rust-lang.org")
        .send(&cdp, Some(&session))
        .await?;
    
    let mut events = page::events::LoadEventFired::subscribe(&cdp);
    if let Some(event) = events.next().await {
        println!("Page loaded at {}", event.timestamp);
    }
    
    Ok(())
}
```

## Installation

```toml
[dependencies]
cdpkit = "0.2"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

## Documentation

- **[Examples](cdpkit/examples/)** - Working code examples

## Code Generation

cdpkit uses auto-generated bindings from the official Chrome DevTools Protocol specification. The `cdpkit_codegen` tool handles this process.

### Regenerate CDP Bindings

To update CDP bindings to the latest protocol version:

```bash
# Run the code generator
cargo run -p cdpkit_codegen

# The generated code will be written to cdpkit/src/protocol.rs
```

### How It Works

1. **Fetch Protocol** - Downloads the latest CDP protocol JSON from Chrome's repository
2. **Parse Specification** - Parses the protocol definition into Rust data structures
3. **Generate Code** - Generates type-safe Rust code for all CDP domains, commands, and events
4. **Output** - Writes the generated code to `cdpkit/src/protocol.rs`

The generated code includes:
- All CDP domains (Page, Network, Runtime, etc.)
- Strongly-typed command structures with builder patterns in `methods` submodule
- Response types in `responses` submodule for method return values
- Event types in `events` submodule for subscription
- Type definitions in `types` submodule for parameters and shared types

### When to Regenerate

- When Chrome releases a new CDP version
- When you need experimental CDP features
- When contributing updates to the protocol bindings

**Note:** The generated `protocol.rs` file is checked into version control, so users don't need to run the generator unless they want to update the protocol version.

## Why cdpkit?

### Direct Protocol Access

cdpkit provides direct access to CDP, giving you full control over browser behavior:

```rust
// Send CDP commands directly with all parameters
page::methods::Navigate::new("https://example.com")
    .with_referrer("https://google.com")
    .with_transition_type(page::types::TransitionType::Link)
    .send(&cdp, Some(&session))
    .await?;

// Parse enum values from strings using FromStr
let transition: page::types::TransitionType = "link".parse()
    .expect("invalid transition type");

// Convert enum values back to strings using AsRef<str>
let s: &str = page::types::TransitionType::Link.as_ref(); // "link"

// Access complete return data
let result = runtime::methods::Evaluate::new("document.title")
    .with_return_by_value(true)
    .send(&cdp, Some(&session))
    .await?;
```

### Powerful Event Handling

Stream-based event system with composition, filtering, and multiplexing:

```rust
use futures::StreamExt;

// Subscribe to multiple events
let mut load_events = page::events::LoadEventFired::subscribe(&cdp);
let mut nav_events = page::events::FrameNavigated::subscribe(&cdp);

// Use stream combinators
let mut combined = futures::stream::select(load_events, nav_events);

// Filter and process
while let Some(event) = combined.next().await {
    // Handle events
}
```

### Compile-Time Type Safety

All CDP operations are type-checked, catching errors at compile time:

```rust
// ✅ Type-checked: required parameters
let cmd = page::methods::Navigate::new("https://example.com");

// ✅ Type-checked: return values
let result = cmd.send(&cdp, Some(&session)).await?;
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

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## References
- [cdp-use](https://github.com/browser-use/cdp-use)
- [chromiumoxide](https://github.com/mattsse/chromiumoxide)
