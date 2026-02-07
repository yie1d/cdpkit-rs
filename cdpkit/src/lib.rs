mod error;
mod inner;
mod listeners;
mod types;

// Generated CDP protocol definitions
#[allow(clippy::all)]
pub mod protocol;

pub use error::CdpError;
pub use types::Method;

// Re-export all CDP domains
pub use protocol::*;

use inner::CDPInner;
use std::sync::Arc;

/// Chrome DevTools Protocol client
pub struct CDP {
    pub(crate) inner: Arc<CDPInner>,
}

impl CDP {
    /// Connect to Chrome by host and port (most common usage)
    ///
    /// Automatically discovers the WebSocket URL from Chrome's debugging endpoint.
    ///
    /// # Arguments
    /// * `host` - The host and port (e.g., "localhost:9222" or "http://localhost:9222")
    ///
    /// # Example
    /// ```no_run
    /// # use cdpkit::CDP;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Connect to default Chrome debugging port
    /// let cdp = CDP::connect("localhost:9222").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(host: &str) -> Result<Self, CdpError> {
        // If it's a WebSocket URL, connect directly
        if host.starts_with("ws://") || host.starts_with("wss://") {
            return Self::connect_ws(host).await;
        }

        // Otherwise, auto-discover the WebSocket URL
        let ws_url = discover_ws_url(host).await?;
        Self::connect_ws(&ws_url).await
    }

    /// Connect directly using a WebSocket URL (advanced usage)
    ///
    /// Use this when you already have the full WebSocket URL.
    ///
    /// # Example
    /// ```no_run
    /// # use cdpkit::CDP;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cdp = CDP::connect_ws("ws://localhost:9222/devtools/browser/...").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_ws(url: &str) -> Result<Self, CdpError> {
        let inner = CDPInner::connect(url).await?;
        Ok(Self { inner })
    }

    /// Send a method
    pub async fn send<C: Method>(
        &self,
        cmd: C,
        session_id: Option<&str>,
    ) -> Result<C::Response, CdpError> {
        self.inner.send_command(cmd, session_id).await
    }

    /// Get the CDP protocol version this library was built with
    pub fn version() -> &'static str {
        CDP_VERSION
    }
}

/// Discover WebSocket URL from Chrome's remote debugging endpoint
async fn discover_ws_url(host: &str) -> Result<String, CdpError> {
    use serde_json::Value;

    // Normalize host to include http:// if not present
    let base_url = if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("http://{}", host)
    };

    // Fetch the JSON endpoint
    let url = format!("{}/json/version", base_url);
    let response = reqwest::get(&url)
        .await
        .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    let json: Value = response
        .json()
        .await
        .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    // Extract webSocketDebuggerUrl
    json.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CdpError::ConnectionFailed("No webSocketDebuggerUrl found".to_string()))
}
