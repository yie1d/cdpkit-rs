use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CdpError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Protocol error {code}: {message}")]
    Protocol { code: i64, message: String },

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Response channel closed")]
    ChannelClosed,

    #[error("Unsupported configuration: {0}")]
    UnsupportedConfiguration(String),

    #[error("Command timed out")]
    Timeout,

    /// TCP connection failed or I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// HTTP discovery phase timed out (/json/version request)
    #[error("Discovery timed out: Chrome did not respond to /json/version in time")]
    DiscoveryTimeout,

    /// WebSocket handshake timed out
    #[error("WebSocket handshake timed out")]
    HandshakeTimeout,

    /// HTTP discovery returned non-200 status code
    #[error("HTTP discovery returned status {0}")]
    HttpStatus(u16),

    /// Invalid or unsupported discovery target passed to `CDP::connect`
    #[error("Invalid discovery input: {0}")]
    InvalidDiscoveryInput(String),

    /// HTTP discovery response format invalid (cannot parse or missing webSocketDebuggerUrl)
    #[error("Invalid discovery response: {0}")]
    InvalidDiscoveryResponse(String),
}

impl CdpError {
    /// Create a new protocol error
    pub fn protocol(code: i64, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
        }
    }

    /// Check if this is any kind of timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::DiscoveryTimeout | Self::HandshakeTimeout
        )
    }

    /// Returns `true` for all connection-phase failures.
    ///
    /// This includes WebSocket-level errors (handshake failures, protocol violations),
    /// I/O errors, discovery timeouts, handshake timeouts, and HTTP status errors.
    ///
    /// Note: `DiscoveryTimeout` and `HandshakeTimeout` also return `true` for [`is_timeout()`](Self::is_timeout),
    /// so checking `is_connection_failed()` subsumes `is_timeout()` for connection errors.
    pub fn is_connection_failed(&self) -> bool {
        matches!(
            self,
            Self::WebSocket(_)
                | Self::Io(_)
                | Self::DiscoveryTimeout
                | Self::HandshakeTimeout
                | Self::HttpStatus(_)
                | Self::InvalidDiscoveryInput(_)
                | Self::InvalidDiscoveryResponse(_)
        )
    }

    /// Check if this is a protocol error
    pub fn is_protocol_error(&self) -> bool {
        matches!(self, Self::Protocol { .. })
    }

    /// Get the error code if this is a protocol error
    pub fn error_code(&self) -> Option<i64> {
        match self {
            Self::Protocol { code, .. } => Some(*code),
            _ => None,
        }
    }
}
