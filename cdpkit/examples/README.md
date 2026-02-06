# cdpkit Examples

This directory contains examples demonstrating various features of cdpkit.

## Prerequisites

1. Launch Chrome with remote debugging enabled:

```bash
# Linux/Mac
google-chrome --remote-debugging-port=9222

# Windows
chrome.exe --remote-debugging-port=9222

# Headless mode
google-chrome --headless --remote-debugging-port=9222
```

2. Run examples (WebSocket URL is auto-discovered):

```bash
# Use default localhost:9222
cargo run --example basic

# Or specify custom host
CDP_HOST="localhost:9222" cargo run --example basic

# Advanced: Use direct WebSocket URL
# (modify example code to use CDP::connect_ws())
```

## Examples

### auto_connect.rs
Demonstrates connection methods.

```bash
cargo run --example auto_connect
```

Demonstrates:
- Connecting by host:port (default method)
- Auto-discovery of WebSocket URL
- Using `CDP::connect()` and `CDP::connect_ws()`

### basic.rs
Basic connection and navigation example.

```bash
cargo run --example basic
```

Demonstrates:
- Connecting to Chrome
- Creating a new page
- Navigating to a URL
- Listening to page load events

### events.rs
Event handling with multiple event streams.

```bash
cargo run --example events
```

Demonstrates:
- Subscribing to multiple events
- Using `tokio::select!` to handle events concurrently
- Console API events
- Page lifecycle events

### evaluate.rs
JavaScript execution and evaluation.

```bash
cargo run --example evaluate
```

Demonstrates:
- Executing JavaScript code
- Getting return values
- Accessing page properties (title, URL, etc.)

### network.rs
Network request and response monitoring.

```bash
cargo run --example network
```

Demonstrates:
- Enabling network domain
- Monitoring HTTP requests
- Monitoring HTTP responses
- Tracking network activity

### screenshot.rs
Capturing page screenshots.

```bash
cargo run --example screenshot
```

Demonstrates:
- Taking screenshots
- Saving images to disk
- Working with base64-encoded data

### dom.rs
DOM querying and manipulation.

```bash
cargo run --example dom
```

Demonstrates:
- Getting the document
- Querying elements with CSS selectors
- Getting element HTML
- Working with node IDs

## Tips

- **Default usage**: Just run examples, connects to `localhost:9222` automatically
- Set `CDP_HOST` for custom host/port (e.g., `CDP_HOST="192.168.1.100:9222"`)
- For advanced usage, use `CDP::connect_ws()` with full WebSocket URL
- Use `RUST_LOG=debug` for detailed logging
- Examples are minimal and focused on specific features
