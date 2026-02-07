use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::warn;

pub(crate) struct EventListeners {
    listeners: HashMap<String, Vec<Sender<Arc<Value>>>>,
}

impl EventListeners {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn add_listener(&mut self, event_name: &str, sender: Sender<Arc<Value>>) {
        self.listeners
            .entry(event_name.to_string())
            .or_default()
            .push(sender);
    }

    pub fn dispatch(&mut self, event_name: &str, event: Arc<Value>) {
        if let Some(listeners) = self.listeners.get_mut(event_name) {
            listeners.retain(|listener| match listener.try_send(event.clone()) {
                Ok(()) => true,
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    warn!(event = event_name, "Event channel full, dropping event");
                    true // keep listener, just drop this event
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => false, // remove listener
            });
        }
    }
}
