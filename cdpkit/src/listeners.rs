use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) struct EventListeners {
    listeners: HashMap<String, Vec<UnboundedSender<Arc<Value>>>>,
}

impl EventListeners {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn add_listener(&mut self, event_name: &str, sender: UnboundedSender<Arc<Value>>) {
        self.listeners
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(sender);
    }

    pub fn dispatch(&mut self, event_name: &str, event: Arc<Value>) {
        if let Some(listeners) = self.listeners.get_mut(event_name) {
            listeners.retain(|listener| listener.send(event.clone()).is_ok());
        }
    }
}
