use serde::{de::DeserializeOwned, Serialize};

/// Trait for CDP methods (commands)
pub trait Method: Serialize {
    /// Response type for this method
    type Response: DeserializeOwned;

    /// CDP method name (e.g., "Page.navigate")
    const METHOD: &'static str;
}
