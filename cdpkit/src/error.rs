use thiserror::Error;

#[derive(Debug, Error)]
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

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

impl CdpError {
    /// Create a new protocol error
    pub fn protocol(code: i64, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
        }
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
