# cdpkit Examples

This directory contains examples demonstrating various features of cdpkit.

## Prerequisites

Launch Chrome with remote debugging enabled:

```bash
# Linux/Mac
google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile

# Windows
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222 --user-data-dir=%TEMP%\cdp-profile

# Headless mode
google-chrome --headless --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile
```

> **Important:** If Chrome is already running, `--remote-debugging-port` will be ignored (Chrome merges into the existing instance). Use `--user-data-dir` to force a separate instance, or close all Chrome windows/processes first.

## Running Examples

All examples automatically discover the WebSocket URL from Chrome's debugging port.

```bash
# Use default localhost:9222
cargo run --example basic

# Or specify custom host
CDP_HOST="localhost:9222" cargo run --example basic

# Or use a different port
CDP_HOST="localhost:9223" cargo run --example basic
```

## Examples

### basic.rs
Basic connection and navigation.

```bash
cargo run --example basic
```

**Demonstrates:**
- Connecting to Chrome (auto-discovers WebSocket URL)
- Creating a new page
- Attaching to a session
- Navigating to a URL
- Listening to page load events

---

### evaluate.rs
JavaScript execution and evaluation.

```bash
cargo run --example evaluate
```

**Demonstrates:**
- Executing JavaScript code
- Getting return values
- Accessing page properties (title, URL)
- Counting DOM elements

---

### dom.rs
DOM querying and manipulation.

```bash
cargo run --example dom
```

**Demonstrates:**
- Getting the document node
- Querying elements with CSS selectors
- Finding h1 and paragraph elements
- Working with node IDs

---

### events.rs
Event handling with multiple event streams.

```bash
cargo run --example events
```

**Demonstrates:**
- Subscribing to multiple events
- Using `tokio::select!` for concurrent event handling
- Frame navigation events
- Page lifecycle events
- Timeout control

---

### network.rs
Network request and response monitoring.

```bash
cargo run --example network
```

**Demonstrates:**
- Enabling network domain
- Monitoring HTTP requests
- Monitoring HTTP responses
- Tracking network activity

---

### screenshot.rs
Capturing page screenshots.

```bash
cargo run --example screenshot
```

**Demonstrates:**
- Taking page screenshots
- Saving PNG images to disk
- Working with base64-encoded data

The screenshot will be saved as `screenshot.png` in the current directory.

---

## Tips

- **Auto-discovery**: Examples automatically connect to `localhost:9222`
- **Custom host**: Set `CDP_HOST` environment variable for different host/port
- **Logging**: Use `RUST_LOG=debug` for detailed logging
- **Clean code**: Examples are minimal and focused on specific features
- **Error handling**: All examples include proper error handling

## Common Issues

- **`Io` / connection refused:** verify that Chrome is running with remote
  debugging enabled and that `CDP_HOST` uses the same host and port.
- **`DiscoveryTimeout`:** the endpoint did not answer `/json/version` before
  the discovery timeout. Check the port and local firewall rules.
- **`HttpStatus` with the Chrome 136+ user-profile toggle:** toggle mode exposes
  a complete WebSocket URL through `DevToolsActivePort` instead of the HTTP
  discovery endpoint. Use `CDP::connect_ws` or
  `CDP::connect_ws_with_timeout`; see the existing-Chrome guide linked from the
  repository README.
