mod error;
mod inner;
mod listeners;
mod types;

// Generated CDP protocol definitions
#[allow(clippy::all)]
#[allow(deprecated)]
pub mod protocol;

pub use error::CdpError;
pub use types::Method;

// Re-export all CDP domains
pub use protocol::*;

use inner::CDPInner;
use std::sync::Arc;
use std::time::Duration;

/// Type alias for CDP event streams returned by `subscribe()`.
pub type EventStream<T> = std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>;

/// Sealed trait module — prevents external implementations of [`Sender`].
#[allow(private_interfaces)]
mod sealed {
    use crate::inner::CDPInner;
    use std::sync::Arc;

    pub trait Sealed {
        fn session_id(&self) -> Option<&str>;
        fn inner(&self) -> &Arc<CDPInner>;
    }

    impl Sealed for crate::CDP {
        fn session_id(&self) -> Option<&str> {
            None
        }
        fn inner(&self) -> &Arc<CDPInner> {
            &self.inner
        }
    }

    impl Sealed for crate::Session<'_> {
        fn session_id(&self) -> Option<&str> {
            Some(&self.session_id)
        }
        fn inner(&self) -> &Arc<CDPInner> {
            &self.cdp.inner
        }
    }

    impl Sealed for crate::OwnedSession {
        fn session_id(&self) -> Option<&str> {
            Some(&self.session_id)
        }
        fn inner(&self) -> &Arc<CDPInner> {
            &self.cdp.inner
        }
    }
}

/// Trait for types that can send CDP commands and subscribe to events.
///
/// Implemented by:
/// - [`CDP`] — sends at browser level (no session)
/// - [`Session`] — sends within a specific session (borrowed)
/// - [`OwnedSession`] — sends within a specific session (owned, `Send + 'static`)
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a CDP command sender",
    label = "expected `&CDP`, `&Session`, or `&OwnedSession`",
    note = "use `cdp.session(session_id)` to create a Session"
)]
pub trait Sender: sealed::Sealed {
    /// Send a typed CDP command.
    fn send_cmd<C: Method>(
        &self,
        cmd: C,
    ) -> impl std::future::Future<Output = Result<C::Response, CdpError>> + Send
    where
        Self: Sync,
    {
        async move {
            sealed::Sealed::inner(self)
                .send_command(cmd, sealed::Sealed::session_id(self))
                .await
        }
    }

    /// Send a raw CDP command by method name and JSON params.
    fn send_raw(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, CdpError>> + Send
    where
        Self: Sync,
    {
        async move {
            sealed::Sealed::inner(self)
                .send_raw(method, params, sealed::Sealed::session_id(self))
                .await
        }
    }

    /// Subscribe to a CDP event stream.
    fn event_stream<T>(&self, event_name: &str) -> EventStream<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        sealed::Sealed::inner(self).event_stream(
            event_name,
            sealed::Sealed::session_id(self).map(str::to_owned),
        )
    }
}

impl Sender for CDP {}
impl Sender for Session<'_> {}
impl Sender for OwnedSession {}

/// Chrome DevTools Protocol client (browser-level connection).
pub struct CDP {
    pub(crate) inner: Arc<CDPInner>,
}

impl Clone for CDP {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// A CDP session bound to a specific target (borrowed).
///
/// Created via [`CDP::session()`]. Cannot be sent across `tokio::spawn` boundaries.
/// Use [`OwnedSession`] if you need `'static` lifetime.
pub struct Session<'a> {
    cdp: &'a CDP,
    session_id: String,
}

impl Session<'_> {
    /// Get the session ID.
    pub fn id(&self) -> &str {
        &self.session_id
    }
}

/// An owned CDP session that can be sent across task boundaries.
///
/// Created via [`CDP::owned_session()`]. Holds a clone of the `CDP` handle
/// (which is cheap — just an Arc increment) so it satisfies `Send + 'static`.
pub struct OwnedSession {
    cdp: CDP,
    session_id: String,
}

impl OwnedSession {
    /// Get the session ID.
    pub fn id(&self) -> &str {
        &self.session_id
    }
}

impl CDP {
    /// Connect to Chrome by host and port (most common usage).
    ///
    /// Automatically discovers the WebSocket URL from Chrome's debugging endpoint.
    ///
    /// # Example
    /// ```no_run
    /// # use cdpkit::CDP;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cdp = CDP::connect("localhost:9222").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(host: &str) -> Result<Self, CdpError> {
        if host.starts_with("ws://") || host.starts_with("wss://") {
            return Self::connect_ws(host).await;
        }
        let ws_url = discover_ws_url(host).await?;
        Self::connect_ws(&ws_url).await
    }

    /// Connect directly using a WebSocket URL (advanced usage).
    pub async fn connect_ws(url: &str) -> Result<Self, CdpError> {
        let inner = CDPInner::connect(url).await?;
        Ok(Self { inner })
    }

    /// Get the CDP protocol version this library was built with.
    pub fn version() -> &'static str {
        CDP_VERSION
    }

    /// Gracefully close the CDP connection.
    pub async fn close(&self) {
        self.inner.close().await;
    }

    /// Check if the connection has been closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Create a borrowed session bound to a specific target.
    pub fn session(&self, session_id: impl Into<String>) -> Session<'_> {
        Session {
            cdp: self,
            session_id: session_id.into(),
        }
    }

    /// Create an owned session that can be sent across task boundaries.
    pub fn owned_session(&self, session_id: impl Into<String>) -> OwnedSession {
        OwnedSession {
            cdp: self.clone(),
            session_id: session_id.into(),
        }
    }

    /// Set the command timeout duration (default: 30 seconds).
    pub fn set_command_timeout(&self, timeout: Duration) {
        self.inner.set_command_timeout(timeout);
    }
}

/// Discover WebSocket URL from Chrome's remote debugging endpoint
async fn discover_ws_url(host: &str) -> Result<String, CdpError> {
    const DISCOVER_TIMEOUT: Duration = Duration::from_secs(10);

    tokio::time::timeout(DISCOVER_TIMEOUT, discover_ws_url_inner(host))
        .await
        .map_err(|_| {
            CdpError::ConnectionFailed(
                "Timed out connecting to Chrome. Make sure Chrome is running with --remote-debugging-port".to_string(),
            )
        })?
}

async fn discover_ws_url_inner(host: &str) -> Result<String, CdpError> {
    use serde_json::Value;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    const MAX_CONTENT_LENGTH: usize = 1_048_576;
    const MAX_HEADER_LINES: usize = 100;

    let addr = if host.starts_with("http://") {
        host.strip_prefix("http://").unwrap()
    } else if host.starts_with("https://") {
        host.strip_prefix("https://").unwrap()
    } else {
        host
    };

    let addr = if addr.contains(':') {
        addr.to_string()
    } else {
        format!("{}:80", addr)
    };

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    let (reader, mut writer) = stream.into_split();

    let request = format!(
        "GET /json/version HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        addr
    );
    writer
        .write_all(request.as_bytes())
        .await
        .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    let mut reader = BufReader::new(reader);
    let mut content_length: usize = 0;
    let mut status_checked = false;
    let mut header_count = 0;

    // Read headers
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

        if line == "\r\n" || line.is_empty() {
            break;
        }

        header_count += 1;
        if header_count > MAX_HEADER_LINES {
            return Err(CdpError::ConnectionFailed(
                "Too many HTTP headers in response".to_string(),
            ));
        }

        if line.len() > 8192 {
            return Err(CdpError::ConnectionFailed(
                "HTTP header line too long".to_string(),
            ));
        }

        if !status_checked {
            status_checked = true;
            if let Some(status) = line.split_whitespace().nth(1) {
                if status != "200" {
                    return Err(CdpError::ConnectionFailed(format!(
                        "Chrome returned HTTP {}. Make sure Chrome is running with --remote-debugging-port",
                        status
                    )));
                }
            }
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            content_length = val.trim().parse().map_err(|e| {
                CdpError::ConnectionFailed(format!("Invalid Content-Length: {}", e))
            })?;
        }
    }

    if content_length == 0 {
        return Err(CdpError::ConnectionFailed(
            "No Content-Length in response. The server may not support HTTP/1.1 or is not a Chrome debugging endpoint".to_string(),
        ));
    }

    if content_length > MAX_CONTENT_LENGTH {
        return Err(CdpError::ConnectionFailed(format!(
            "Response too large ({} bytes). Expected a small JSON response from Chrome",
            content_length
        )));
    }

    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    let json: Value =
        serde_json::from_slice(&body).map_err(|e| CdpError::ConnectionFailed(e.to_string()))?;

    json.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CdpError::ConnectionFailed(
                "No webSocketDebuggerUrl found in Chrome's response. Make sure Chrome is running with --remote-debugging-port".to_string(),
            )
        })
}
