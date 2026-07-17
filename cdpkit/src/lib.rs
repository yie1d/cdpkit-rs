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
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

/// Reason why the CDP connection was closed.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReason {
    /// Graceful close initiated by the client
    Normal,
    /// Remote end closed the connection
    Remote,
    /// Connection lost due to an error
    Error(String),
}

/// Default timeout for the WebSocket handshake during connection (30 seconds).
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Type alias for CDP event streams returned by `subscribe()`.
pub type EventStream<T> = std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>;

/// Shared statistics for one event subscription.
#[derive(Clone, Debug, Default)]
pub struct EventStreamStats {
    dropped_events: Arc<AtomicU64>,
}

impl EventStreamStats {
    /// Number of events dropped because the bounded subscription buffer overflowed.
    pub fn dropped_events(&self) -> u64 {
        self.dropped_events.load(Ordering::Relaxed)
    }

    pub(crate) fn record_drop(&self) {
        self.dropped_events.fetch_add(1, Ordering::Relaxed);
    }
}

/// CDP event stream that surfaces deserialization and overflow errors.
pub struct EventStreamResult<T> {
    inner: Pin<Box<dyn futures::Stream<Item = Result<T, CdpError>> + Send>>,
    stats: EventStreamStats,
}

impl<T> EventStreamResult<T> {
    pub(crate) fn new(
        inner: Pin<Box<dyn futures::Stream<Item = Result<T, CdpError>> + Send>>,
        stats: EventStreamStats,
    ) -> Self {
        Self { inner, stats }
    }

    /// Return a cloneable handle to this subscription's overflow statistics.
    pub fn stats(&self) -> EventStreamStats {
        self.stats.clone()
    }
}

impl<T> futures::Stream for EventStreamResult<T> {
    type Item = Result<T, CdpError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Overflow behavior for explicitly bounded event streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOverflowStrategy {
    /// Drop the incoming event when the per-subscriber buffer is already full.
    DropNewest,
    /// Close the subscriber stream when the per-subscriber buffer overflows.
    CloseStream,
}

/// Buffering policy for event subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamPolicy {
    /// Preserve the historical behavior: unbounded per-subscription buffering.
    Unbounded,
    /// Bound the per-subscription buffer and apply an explicit overflow strategy.
    Bounded {
        capacity: NonZeroUsize,
        overflow: EventOverflowStrategy,
    },
}

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
            cmd.validate()?;
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
        self.event_stream_with_policy(event_name, EventStreamPolicy::Unbounded)
    }

    /// Subscribe to a CDP event stream with an explicit buffering policy.
    fn event_stream_with_policy<T>(
        &self,
        event_name: &str,
        policy: EventStreamPolicy,
    ) -> EventStream<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        sealed::Sealed::inner(self).event_stream(
            event_name,
            sealed::Sealed::session_id(self).map(str::to_owned),
            policy,
        )
    }

    /// Subscribe to a CDP event stream and surface deserialization errors as `Result`.
    fn event_stream_result<T>(&self, event_name: &str) -> EventStreamResult<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        self.event_stream_result_with_policy(event_name, EventStreamPolicy::Unbounded)
    }

    /// Subscribe to a CDP event stream with explicit buffering and explicit decode errors.
    fn event_stream_result_with_policy<T>(
        &self,
        event_name: &str,
        policy: EventStreamPolicy,
    ) -> EventStreamResult<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        sealed::Sealed::inner(self).event_stream_result(
            event_name,
            sealed::Sealed::session_id(self).map(str::to_owned),
            policy,
        )
    }
}

impl Sender for CDP {}
impl Sender for Session<'_> {}
impl Sender for OwnedSession {}

struct DiscoveryEndpoint {
    connect_addr: String,
    host_header: String,
}

fn parse_discovery_endpoint(input: &str) -> Result<DiscoveryEndpoint, CdpError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CdpError::InvalidDiscoveryInput(
            "expected host:port or http://host:port".to_string(),
        ));
    }

    let authority = if let Some(authority) = trimmed.strip_prefix("http://") {
        authority
    } else if trimmed.starts_with("https://") {
        return Err(CdpError::InvalidDiscoveryInput(
            "https:// discovery is not supported; use http://host:port or host:port".to_string(),
        ));
    } else if trimmed.contains("://") {
        return Err(CdpError::InvalidDiscoveryInput(
            "unsupported discovery scheme; use http://host:port or host:port".to_string(),
        ));
    } else {
        trimmed
    };

    if authority.contains('/') || authority.contains('?') || authority.contains('#') {
        return Err(CdpError::InvalidDiscoveryInput(
            "discovery only accepts a host:port authority and always queries /json/version"
                .to_string(),
        ));
    }

    let (host, port) = if authority.starts_with('[') {
        let end = authority.find("]:").ok_or_else(|| {
            CdpError::InvalidDiscoveryInput(
                "IPv6 discovery targets must be written as [host]:port".to_string(),
            )
        })?;
        (&authority[..=end], &authority[end + 2..])
    } else {
        authority.split_once(':').ok_or_else(|| {
            CdpError::InvalidDiscoveryInput(
                "discovery requires an explicit host:port; the port is not inferred".to_string(),
            )
        })?
    };

    if host.is_empty() {
        return Err(CdpError::InvalidDiscoveryInput(
            "discovery host cannot be empty".to_string(),
        ));
    }

    let port: u16 = port.parse().map_err(|_| {
        CdpError::InvalidDiscoveryInput("discovery port must be a valid u16".to_string())
    })?;

    Ok(DiscoveryEndpoint {
        connect_addr: format!("{host}:{port}"),
        host_header: format!("{host}:{port}"),
    })
}

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

impl Drop for CDP {
    fn drop(&mut self) {
        // When the last CDP handle is dropped, trigger connection shutdown
        // so the background message loop task doesn't leak the WebSocket.
        if Arc::strong_count(&self.inner) == 1 {
            // Last handle — send a close frame non-blocking.
            // If the channel is already closed or the loop already exited, ignore the error.
            self.inner.try_close();
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
    /// Connect to Chrome through HTTP discovery (most common usage).
    ///
    /// Accepts only `host:port` or `http://host:port`, then discovers the WebSocket URL
    /// from `/json/version`. Use [`CDP::connect_ws`] for complete `ws://` / `wss://` URLs.
    /// Uses a default WebSocket handshake timeout of 30 seconds.
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
        Self::connect_with_timeout(host, DEFAULT_CONNECT_TIMEOUT).await
    }

    /// Connect through HTTP discovery with a custom WebSocket handshake timeout.
    ///
    /// Accepts only `host:port` or `http://host:port`. Use
    /// [`CDP::connect_ws_with_timeout`] for complete `ws://` / `wss://` URLs.
    /// The `timeout` controls how long to wait for the WebSocket handshake to complete.
    /// The HTTP discovery phase (`/json/version`) has its own independent 10s timeout.
    ///
    /// # Example
    /// ```no_run
    /// # use cdpkit::CDP;
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cdp = CDP::connect_with_timeout("localhost:9222", Duration::from_secs(10)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_with_timeout(host: &str, timeout: Duration) -> Result<Self, CdpError> {
        let ws_url = discover_ws_url(host).await?;
        Self::connect_ws_with_timeout(&ws_url, timeout).await
    }

    /// Connect directly using a complete `ws://` or `wss://` WebSocket URL.
    ///
    /// Uses a default WebSocket handshake timeout of 30 seconds.
    pub async fn connect_ws(url: &str) -> Result<Self, CdpError> {
        Self::connect_ws_with_timeout(url, DEFAULT_CONNECT_TIMEOUT).await
    }

    /// Connect directly using a complete `ws://` or `wss://` URL with a custom handshake timeout.
    ///
    /// The `timeout` controls how long to wait for the WebSocket handshake to complete.
    /// If the handshake does not finish within the given duration, returns
    /// [`CdpError::HandshakeTimeout`].
    ///
    /// # Example
    /// ```no_run
    /// # use cdpkit::CDP;
    /// # use std::time::Duration;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cdp = CDP::connect_ws_with_timeout("ws://127.0.0.1:9222/devtools/browser/xxx", Duration::from_secs(5)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_ws_with_timeout(url: &str, timeout: Duration) -> Result<Self, CdpError> {
        let inner = CDPInner::connect(url, timeout).await?;
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

    /// Get the reason why the connection was closed, if it has been closed.
    pub fn close_reason(&self) -> Option<CloseReason> {
        self.inner.close_reason()
    }

    /// Resolves when the connection is closed.
    /// Returns immediately if already closed.
    /// Useful for spawning a monitoring task:
    ///
    /// ```rust,no_run
    /// # use cdpkit::CDP;
    /// # async fn example(cdp: CDP) {
    /// tokio::spawn(async move {
    ///     cdp.closed().await;
    ///     // cleanup...
    /// });
    /// # }
    /// ```
    pub async fn closed(&self) {
        self.inner.closed().await;
    }
}

/// Discover WebSocket URL from Chrome's remote debugging endpoint
async fn discover_ws_url(host: &str) -> Result<String, CdpError> {
    const DISCOVER_TIMEOUT: Duration = Duration::from_secs(10);

    tokio::time::timeout(DISCOVER_TIMEOUT, discover_ws_url_inner(host))
        .await
        .map_err(|_| CdpError::DiscoveryTimeout)?
}

async fn discover_ws_url_inner(host: &str) -> Result<String, CdpError> {
    use serde_json::Value;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    const MAX_CONTENT_LENGTH: usize = 1_048_576;
    const MAX_HEADER_LINES: usize = 100;

    let endpoint = parse_discovery_endpoint(host)?;

    let stream = TcpStream::connect(&endpoint.connect_addr)
        .await
        .map_err(|e| CdpError::Io(e.to_string()))?;

    let (reader, mut writer) = stream.into_split();

    let request = format!(
        "GET /json/version HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        endpoint.host_header
    );
    writer
        .write_all(request.as_bytes())
        .await
        .map_err(|e| CdpError::Io(e.to_string()))?;

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
            .map_err(|e| CdpError::Io(e.to_string()))?;

        if line == "\r\n" || line.is_empty() {
            break;
        }

        header_count += 1;
        if header_count > MAX_HEADER_LINES {
            return Err(CdpError::InvalidDiscoveryResponse(
                "Too many HTTP headers in response".to_string(),
            ));
        }

        if line.len() > 8192 {
            return Err(CdpError::InvalidDiscoveryResponse(
                "HTTP header line too long".to_string(),
            ));
        }

        if !status_checked {
            status_checked = true;
            if let Some(status_str) = line.split_whitespace().nth(1) {
                if status_str != "200" {
                    let code = status_str.parse::<u16>().map_err(|_| {
                        CdpError::InvalidDiscoveryResponse("Invalid HTTP status line".to_string())
                    })?;
                    return Err(CdpError::HttpStatus(code));
                }
            }
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            content_length = val.trim().parse().map_err(|e| {
                CdpError::InvalidDiscoveryResponse(format!("Invalid Content-Length: {}", e))
            })?;
        }
    }

    if content_length == 0 {
        return Err(CdpError::InvalidDiscoveryResponse(
            "No Content-Length in response".to_string(),
        ));
    }

    if content_length > MAX_CONTENT_LENGTH {
        return Err(CdpError::InvalidDiscoveryResponse(format!(
            "Response too large ({} bytes)",
            content_length
        )));
    }

    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| CdpError::Io(e.to_string()))?;

    let json: Value = serde_json::from_slice(&body)
        .map_err(|e| CdpError::InvalidDiscoveryResponse(format!("Failed to parse JSON: {}", e)))?;

    json.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CdpError::InvalidDiscoveryResponse(
                "No webSocketDebuggerUrl found in response".to_string(),
            )
        })
}
