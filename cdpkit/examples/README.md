# cdpkit Examples

This directory contains examples demonstrating various features of cdpkit.

## Prerequisites

Launch Chrome with remote debugging enabled:

```bash
# Linux/Mac
google-chrome --remote-debugging-port=9222

# Windows
chrome.exe --remote-debugging-port=9222

# Headless mode
google-chrome --headless --remote-debugging-port=9222
```

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

**Output:**
```
Connected to Chrome
Created target: ...
Attached to session: ...
Navigating to https://example.com
Page loaded at timestamp: ...
```

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

**Output:**
```
Connected to Chrome
Page loaded
Page title: "Example Domain"
Page URL: "https://example.com/"
Total elements: 11
```

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

**Output:**
```
Connected to Chrome
Navigating to https://example.com
Page loaded
Document node ID: 1
Found h1 element with node ID: 7
Found 2 paragraph elements
```

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

**Output:**
```
Connected to Chrome
Navigating to https://example.com
Listening to events...
Frame navigated: "https://example.com/"
Page loaded at: ...
Timeout reached
```

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

**Output:**
```
Connected to Chrome
Navigating to https://example.com
[Request #1] GET https://example.com/
[Response #1] https://example.com/ - Status: 200

Page loaded!
Total requests: 1
Total responses: 1
```

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

**Output:**
```
Connected to Chrome
Navigating to https://example.com
Page loaded
Capturing screenshot...
Screenshot saved to screenshot.png (60700 bytes)
```

The screenshot will be saved as `screenshot.png` in the current directory.

---

## Tips

- **Auto-discovery**: Examples automatically connect to `localhost:9222`
- **Custom host**: Set `CDP_HOST` environment variable for different host/port
- **Logging**: Use `RUST_LOG=debug` for detailed logging
- **Clean code**: Examples are minimal and focused on specific features
- **Error handling**: All examples include proper error handling

## Common Issues

**Chrome not found:**
```
Error: Connection failed
```
→ Make sure Chrome is running with `--remote-debugging-port=9222`

**Port already in use:**
```
Error: Address already in use
```
→ Use a different port: `google-chrome --remote-debugging-port=9223`

**Permission denied:**
```
Error: Permission denied
```
→ On Linux, you may need to run Chrome with `--no-sandbox` in some environments
