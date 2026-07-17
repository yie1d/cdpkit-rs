use crate::error::CdpError;
use crate::listeners::{EventListeners, EventReceiver};
use crate::types::Method;
use crate::{EventStreamPolicy, EventStreamResult, EventStreamStats};
use futures::stream::Stream;
use futures::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, trace, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type RawEventStream = Pin<Box<dyn Stream<Item = Result<Arc<Value>, CdpError>> + Send>>;

/// Channel capacity for outgoing WebSocket messages
const WS_SEND_CAPACITY: usize = 256;

/// Default timeout for CDP commands
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct CDPInner {
    tx: mpsc::Sender<Message>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, CdpError>>>>,
    event_listeners: Arc<std::sync::RwLock<EventListeners>>,
    next_id: AtomicU64,
    closed: AtomicBool,
    close_complete: watch::Sender<bool>,
    _close_complete_rx: watch::Receiver<bool>,
    command_timeout_ms: AtomicU64,
    close_reason: std::sync::Mutex<Option<crate::CloseReason>>,
}

impl CDPInner {
    pub async fn connect(url: &str, connect_timeout: Duration) -> Result<Arc<Self>, CdpError> {
        info!("Connecting to CDP at {}", url);

        let (ws, _) = tokio::time::timeout(connect_timeout, connect_async(url))
            .await
            .map_err(|_| CdpError::HandshakeTimeout)??;

        info!("Connected to CDP successfully");

        let (tx, rx) = mpsc::channel(WS_SEND_CAPACITY);

        let (close_complete, close_complete_rx) = watch::channel(false);

        let inner = Arc::new(Self {
            tx,
            pending: Mutex::new(HashMap::new()),
            event_listeners: Arc::new(std::sync::RwLock::new(EventListeners::new())),
            next_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            close_complete,
            _close_complete_rx: close_complete_rx,
            command_timeout_ms: AtomicU64::new(DEFAULT_COMMAND_TIMEOUT.as_millis() as u64),
            close_reason: std::sync::Mutex::new(None),
        });

        let inner_clone = inner.clone();
        tokio::spawn(async move {
            inner_clone.message_loop(ws, rx).await;
        });

        Ok(inner)
    }

    pub async fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        // close_reason is set by the message loop based on exit path
        let _ = self.tx.send(Message::Close(None)).await;
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn set_command_timeout(&self, timeout: Duration) {
        self.command_timeout_ms
            .store(timeout.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn close_reason(&self) -> Option<crate::CloseReason> {
        self.close_reason
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub async fn closed(&self) {
        let mut close_complete = self.close_complete.subscribe();
        if *close_complete.borrow() {
            return;
        }
        let _ = close_complete.changed().await;
    }

    /// Non-blocking close attempt used by `Drop for CDP`.
    /// Sends a Close frame if the channel is still open; ignores errors.
    pub(crate) fn try_close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = self.tx.try_send(Message::Close(None));
    }

    fn command_timeout(&self) -> Duration {
        Duration::from_millis(self.command_timeout_ms.load(Ordering::Relaxed))
    }

    pub async fn send_command<C: Method>(
        &self,
        cmd: C,
        session_id: Option<&str>,
    ) -> Result<C::Response, CdpError> {
        let mut msg = serde_json::json!({
            "method": C::METHOD,
            "params": cmd,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = serde_json::json!(sid);
        }

        let response = self.send_message(msg, C::METHOD).await?;

        let response = if response.is_null() || response.as_object().is_some_and(|o| o.is_empty()) {
            Value::Null
        } else {
            response
        };
        serde_json::from_value(response).map_err(Into::into)
    }

    pub async fn send_raw(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value, CdpError> {
        let mut msg = serde_json::json!({
            "method": method,
            "params": params,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = serde_json::json!(sid);
        }

        self.send_message(msg, method).await
    }

    async fn send_message(&self, mut msg: Value, method: &str) -> Result<Value, CdpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        msg["id"] = serde_json::json!(id);

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            if self.closed.load(Ordering::Acquire) {
                return Err(CdpError::ConnectionClosed);
            }
            pending.insert(id, tx);
        }

        debug!(id = id, method = method, "Sending command");
        trace!("Command payload: {}", msg);

        if self
            .tx
            .send(Message::Text(msg.to_string().into()))
            .await
            .is_err()
        {
            self.pending.lock().await.remove(&id);
            return Err(CdpError::ConnectionClosed);
        }

        let response = match tokio::time::timeout(self.command_timeout(), rx).await {
            Ok(Ok(result)) => result?,
            Ok(Err(_)) => return Err(CdpError::ChannelClosed),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                return Err(CdpError::Timeout);
            }
        };

        debug!(id = id, method = method, "Received response");
        trace!("Response payload: {}", response);

        Ok(response)
    }

    pub fn event_stream<T>(
        &self,
        event_name: &str,
        session_id: Option<String>,
        policy: EventStreamPolicy,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let results = self.event_stream_result(event_name, session_id, policy);
        let event_name: Arc<str> = event_name.into();

        Box::pin(results.filter_map(move |event| {
            let event_name = Arc::clone(&event_name);
            async move {
                match event {
                    Ok(event) => Some(event),
                    Err(e) => {
                        warn!(event = %event_name, error = %e, "Failed to deserialize event");
                        None
                    }
                }
            }
        }))
    }

    pub fn event_stream_result<T>(
        &self,
        event_name: &str,
        session_id: Option<String>,
        policy: EventStreamPolicy,
    ) -> EventStreamResult<T>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let event_name: Arc<str> = event_name.into();

        debug!(event = %event_name, "Subscribing to event");

        let receiver = self
            .event_listeners
            .write()
            .unwrap_or_else(|e| {
                warn!("EventListeners RwLock was poisoned, recovering");
                e.into_inner()
            })
            .add_listener(&event_name, session_id, policy);

        let (rx_stream, stats): (RawEventStream, EventStreamStats) = match receiver {
            EventReceiver::Unbounded(rx) => (
                Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(Ok)),
                EventStreamStats::default(),
            ),
            EventReceiver::Bounded {
                receiver,
                overflow,
                capacity,
            } => {
                let stats = overflow.stats();
                let event_name = Arc::clone(&event_name);
                let stream = futures::stream::unfold(
                    (receiver, overflow, false),
                    move |(mut receiver, overflow, overflow_emitted)| {
                        let event_name = Arc::clone(&event_name);
                        async move {
                            if let Some(value) = receiver.recv().await {
                                return Some((Ok(value), (receiver, overflow, overflow_emitted)));
                            }

                            if !overflow_emitted && overflow.closed_by_overflow() {
                                let error = CdpError::EventStreamOverflow {
                                    event: event_name.to_string(),
                                    capacity,
                                    dropped: overflow.stats().dropped_events(),
                                };
                                return Some((Err(error), (receiver, overflow, true)));
                            }

                            None
                        }
                    },
                );
                (Box::pin(stream), stats)
            }
        };

        EventStreamResult::new(
            Box::pin(rx_stream.map(move |value| {
                value.and_then(|value| serde_json::from_value((*value).clone()).map_err(Into::into))
            })),
            stats,
        )
    }

    async fn message_loop(&self, mut ws: WsStream, mut rx: mpsc::Receiver<Message>) {
        debug!("Starting message loop");
        let exit_reason: Option<crate::CloseReason>;
        loop {
            tokio::select! {
                msg = ws.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            trace!("Received message: {}", text);
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                self.handle_message(value).await;
                            } else {
                                warn!("Failed to parse message as JSON: {}", text);
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            info!("WebSocket closed: {:?}", frame);
                            exit_reason = Some(crate::CloseReason::Remote);
                            let _ = ws.close(None).await;
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            exit_reason = Some(crate::CloseReason::Remote);
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            exit_reason = Some(crate::CloseReason::Error(e.to_string()));
                            break;
                        }
                        _ => {}
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = ws.send(msg).await {
                                error!("Failed to send message: {}", e);
                                exit_reason = Some(crate::CloseReason::Error(e.to_string()));
                                break;
                            }
                        }
                        None => {
                            debug!("All senders dropped, shutting down");
                            exit_reason = Some(crate::CloseReason::Normal);
                            break;
                        }
                    }
                }
            }
        }

        debug!("Message loop ended, cleaning up");

        // If `closed` is already true, it means close() (or try_close()) was called by the user
        // before the loop exited — treat the exit as Normal regardless of the WS-level outcome.
        let user_initiated = self.closed.load(Ordering::Acquire);
        self.closed.store(true, Ordering::Release);

        // Record close reason
        if let Some(reason) = exit_reason {
            let effective = if user_initiated {
                crate::CloseReason::Normal
            } else {
                reason
            };
            *self.close_reason.lock().unwrap_or_else(|e| e.into_inner()) = Some(effective);
        }

        let mut pending = self.pending.lock().await;
        for (_, tx) in pending.drain() {
            let _ = tx.send(Err(CdpError::ConnectionClosed));
        }
        drop(pending);

        self.event_listeners
            .write()
            .unwrap_or_else(|e| {
                warn!("EventListeners RwLock was poisoned, recovering");
                e.into_inner()
            })
            .clear();

        let _ = self.close_complete.send(true);
    }

    async fn handle_message(&self, value: Value) {
        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
            if let Some(tx) = self.pending.lock().await.remove(&id) {
                if let Some(error) = value.get("error") {
                    let code = error.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
                    let message = error
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error")
                        .to_string();
                    warn!(id = id, code = code, message = %message, "Command failed");
                    let _ = tx.send(Err(CdpError::Protocol { code, message }));
                } else {
                    let result = value.get("result").cloned().unwrap_or(Value::Null);
                    trace!(id = id, "Command succeeded");
                    let _ = tx.send(Ok(result));
                }
            } else {
                warn!(id = id, "Received response for unknown command ID");
            }
        } else if let Some(method) = value.get("method").and_then(|v| v.as_str()) {
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            let session_id = value.get("sessionId").and_then(|v| v.as_str());
            trace!(event = method, "Dispatching event");
            self.event_listeners
                .write()
                .unwrap_or_else(|e| {
                    warn!("EventListeners RwLock was poisoned, recovering");
                    e.into_inner()
                })
                .dispatch(method, session_id, Arc::new(params));
        } else {
            warn!("Received message without id or method: {}", value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::SinkExt;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn closed_waits_until_listener_cleanup_finishes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (trigger_tx, trigger_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            trigger_rx.await.unwrap();
            ws.send(Message::Close(None)).await.unwrap();
        });

        let inner = CDPInner::connect(
            &format!("ws://127.0.0.1:{}", addr.port()),
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        let mut events =
            inner.event_stream_result::<Value>("Test.event", None, EventStreamPolicy::Unbounded);

        let mut closed = Box::pin(inner.closed());
        assert!(futures::poll!(&mut closed).is_pending());

        let event_listeners = Arc::clone(&inner.event_listeners);
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let lock_holder = std::thread::spawn(move || {
            let _listeners = event_listeners.write().unwrap_or_else(|e| e.into_inner());
            locked_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        locked_rx.recv().unwrap();
        trigger_tx.send(()).unwrap();

        while !inner.is_closed() {
            tokio::task::yield_now().await;
        }

        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut closed)
                .await
                .is_err(),
            "closed() resolved before listener cleanup could acquire its lock"
        );

        release_tx.send(()).unwrap();
        lock_holder.join().unwrap();
        tokio::time::timeout(Duration::from_secs(2), &mut closed)
            .await
            .unwrap();
        server.await.unwrap();

        assert!(inner.close_reason().is_some());
        assert!(inner.pending.lock().await.is_empty());
        assert!(events.next().await.is_none());
    }
}
