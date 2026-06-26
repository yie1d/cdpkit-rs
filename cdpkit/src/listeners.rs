use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

struct Listener {
    session_id: Option<String>,
    sender: UnboundedSender<Arc<Value>>,
}

pub(crate) struct EventListeners {
    listeners: HashMap<String, Vec<Listener>>,
}

impl EventListeners {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn add_listener(
        &mut self,
        event_name: &str,
        session_id: Option<String>,
        sender: UnboundedSender<Arc<Value>>,
    ) {
        self.listeners
            .entry(event_name.to_string())
            .or_default()
            .push(Listener { session_id, sender });
    }

    pub fn dispatch(&mut self, event_name: &str, session_id: Option<&str>, event: Arc<Value>) {
        if let Some(listeners) = self.listeners.get_mut(event_name) {
            listeners.retain(|listener| {
                let matches = match (&listener.session_id, session_id) {
                    (None, _) => true,
                    (Some(expected), Some(actual)) => expected == actual,
                    (Some(_), None) => false,
                };

                if !matches {
                    return true;
                }

                // UnboundedSender::send only fails when the receiver is dropped
                listener.sender.send(event.clone()).is_ok()
            });
        }
    }

    pub fn clear(&mut self) {
        self.listeners.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::mpsc;

    #[test]
    fn dispatch_to_multiple_listeners() {
        let mut listeners = EventListeners::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        listeners.add_listener("Test.event", None, tx1);
        listeners.add_listener("Test.event", None, tx2);

        let event = Arc::new(json!({"key": "value"}));
        listeners.dispatch("Test.event", None, event.clone());

        assert_eq!(*rx1.try_recv().unwrap(), json!({"key": "value"}));
        assert_eq!(*rx2.try_recv().unwrap(), json!({"key": "value"}));
    }

    #[test]
    fn closed_listener_removed() {
        let mut listeners = EventListeners::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, rx2_dropped) = mpsc::unbounded_channel::<Arc<Value>>();

        listeners.add_listener("Test.event", None, tx1);
        listeners.add_listener("Test.event", None, tx2);

        drop(rx2_dropped);

        let event = Arc::new(json!({"data": 1}));
        listeners.dispatch("Test.event", None, event);

        assert_eq!(*rx1.try_recv().unwrap(), json!({"data": 1}));

        let event2 = Arc::new(json!({"data": 2}));
        listeners.dispatch("Test.event", None, event2);
        assert_eq!(*rx1.try_recv().unwrap(), json!({"data": 2}));
    }

    #[test]
    fn unbounded_channel_buffers_all_events() {
        let mut listeners = EventListeners::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        listeners.add_listener("Test.event", None, tx);

        // Unbounded channel never drops events
        for i in 0..100 {
            listeners.dispatch("Test.event", None, Arc::new(json!(i)));
        }

        for i in 0..100 {
            assert_eq!(*rx.try_recv().unwrap(), json!(i));
        }
    }

    #[test]
    fn dispatch_to_nonexistent_event_is_noop() {
        let mut listeners = EventListeners::new();
        listeners.dispatch("NoSuchEvent", None, Arc::new(json!({})));
    }

    #[test]
    fn session_filter_only_matching() {
        let mut listeners = EventListeners::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        listeners.add_listener("Test.event", Some("session-A".into()), tx1);
        listeners.add_listener("Test.event", Some("session-B".into()), tx2);

        listeners.dispatch("Test.event", Some("session-A"), Arc::new(json!({"a": 1})));

        assert_eq!(*rx1.try_recv().unwrap(), json!({"a": 1}));
        assert!(rx2.try_recv().is_err());
    }

    #[test]
    fn none_session_listener_receives_all() {
        let mut listeners = EventListeners::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        listeners.add_listener("Test.event", None, tx);

        listeners.dispatch("Test.event", Some("any-session"), Arc::new(json!(1)));
        listeners.dispatch("Test.event", None, Arc::new(json!(2)));

        assert_eq!(*rx.try_recv().unwrap(), json!(1));
        assert_eq!(*rx.try_recv().unwrap(), json!(2));
    }
}
