use crate::error::CdpError;
use crate::listeners::EventListeners;
use crate::types::Method;
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
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, trace, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Channel capacity for outgoing WebSocket messages
const WS_SEND_CAPACITY: usize = 256;

/// Channel capacity for event listeners
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Default timeout for CDP commands
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct CDPInner {
    tx: mpsc::Sender<Message>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, CdpError>>>>,
    event_listeners: Arc<std::sync::RwLock<EventListeners>>,
    next_id: AtomicU64,
    closed: AtomicBool,
    command_timeout_ms: AtomicU64,
}

impl CDPInner {
    pub async fn connect(url: &str) -> Result<Arc<Self>, CdpError> {
        info!("Connecting to CDP at {}", url);
        let (ws, _) = connect_async(url).await?;
        info!("Connected to CDP successfully");

        let (tx, rx) = mpsc::channel(WS_SEND_CAPACITY);

        let inner = Arc::new(Self {
            tx,
            pending: Mutex::new(HashMap::new()),
            event_listeners: Arc::new(std::sync::RwLock::new(EventListeners::new())),
            next_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            command_timeout_ms: AtomicU64::new(DEFAULT_COMMAND_TIMEOUT.as_millis() as u64),
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
        let _ = self.tx.send(Message::Close(None)).await;
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn set_command_timeout(&self, timeout: Duration) {
        self.command_timeout_ms
            .store(timeout.as_millis() as u64, Ordering::Relaxed);
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
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let event_name: Arc<str> = event_name.into();

        debug!(event = %event_name, "Subscribing to event");

        self.event_listeners
            .write()
            .unwrap_or_else(|e| {
                warn!("EventListeners RwLock was poisoned, recovering");
                e.into_inner()
            })
            .add_listener(&event_name, session_id, tx);

        let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        Box::pin(rx_stream.filter_map(move |v| {
            let event_name = Arc::clone(&event_name);
            async move {
                let value = (*v).clone();
                match serde_json::from_value(value) {
                    Ok(event) => Some(event),
                    Err(e) => {
                        warn!(event = %event_name, error = %e, "Failed to deserialize event");
                        None
                    }
                }
            }
        }))
    }

    async fn message_loop(&self, mut ws: WsStream, mut rx: mpsc::Receiver<Message>) {
        debug!("Starting message loop");
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
                            let _ = ws.close(None).await;
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
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
                                break;
                            }
                        }
                        None => {
                            debug!("All senders dropped, shutting down");
                            break;
                        }
                    }
                }
            }
        }

        debug!("Message loop ended, cleaning up");
        self.closed.store(true, Ordering::Release);

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
