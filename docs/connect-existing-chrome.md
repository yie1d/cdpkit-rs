# Connecting to Your Existing Chrome (Chrome 136+)

Connect to your already-running Chrome with all cookies, logins, and extensions intact. This complements the traditional approach of launching a new instance with `--remote-debugging-port`.

## Prerequisites

- Chrome 136 or later
- Enable remote debugging toggle: open `chrome://inspect/#remote-debugging` and turn on **"Allow remote debugging for this browser instance"**

This setting is persisted in your profile's `Local State` file and survives browser restarts.

## Why not `CDP::connect("localhost:port")`?

When Chrome is started with the toggle (not `--remote-debugging-port`), the HTTP discovery endpoint `/json/version` is disabled (returns 404). The `CDP::connect("host:port")` helper relies on that endpoint to obtain the WebSocket URL, so it will fail.

Instead, you must read the `DevToolsActivePort` file and connect directly via the WebSocket URL.

## Read DevToolsActivePort

Chrome writes a `DevToolsActivePort` file to the user data directory on startup. It contains two lines:

```
<port>
<ws_path>
```

For example:
```
9222
/devtools/browser/b0e3c5a1-...
```

The port may change on every launch — never hard-code it.

### File location by platform

| Platform | Path |
|----------|------|
| Windows  | `%LOCALAPPDATA%\Google\Chrome\User Data\DevToolsActivePort` |
| macOS    | `~/Library/Application Support/Google/Chrome/DevToolsActivePort` |
| Linux    | `~/.config/google-chrome/DevToolsActivePort` |

> For non-default profiles or Chromium-based browsers, the user data directory will differ. Adjust accordingly.

## Connect with cdpkit

```rust
use cdpkit::CDP;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read DevToolsActivePort (adjust path for your OS)
    let path = r"C:\Users\<you>\AppData\Local\Google\Chrome\User Data\DevToolsActivePort";
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();
    let port = lines.next().unwrap().trim();
    let ws_path = lines.next().unwrap_or("").trim();

    // Direct WebSocket connection (bypasses disabled /json/version)
    let ws_url = format!("ws://127.0.0.1:{}{}", port, ws_path);
    let cdp = CDP::connect_with_timeout(&ws_url, Duration::from_secs(10)).await?;

    println!("Connected to existing Chrome!");
    Ok(())
}
```

## Notes

- You are connecting to the user's default BrowserContext — cookies, login sessions, and extensions are all shared.
- `Target::createTarget` without a `browserContextId` opens a new tab in the user's visible browser window.
- DevTools frontend is itself a CDP client. Multiple clients can coexist on the same browser; there is no exclusive lock.
- Use `CDP::connect_with_timeout` rather than plain `connect` to avoid hanging indefinitely if the `DevToolsActivePort` file is stale (pointing to a port from a previous Chrome process that is no longer running).
