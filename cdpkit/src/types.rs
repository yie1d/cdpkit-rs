use crate::{error::CdpError, CDP};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// Trait for CDP commands
pub trait Command: Serialize {
    /// Response type for this command
    type Response: DeserializeOwned;

    /// CDP method name (e.g., "Page.navigate")
    const METHOD: &'static str;

    /// Send this command using fluent API
    fn send(
        self,
        cdp: &CDP,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<Self::Response, CdpError>>
    where
        Self: Sized,
    {
        async move { cdp.send(self, session_id).await }
    }
}
