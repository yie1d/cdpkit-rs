use crate::error::CdpError;
use crate::listeners::EventListeners;
use crate::types::Command;
use futures::stream::Stream;
use futures::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, trace, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) struct CDPInner {
    tx: mpsc::UnboundedSender<Message>,
    pending: RwLock<HashMap<u64, oneshot::Sender<Result<Value, CdpError>>>>,
    event_listeners: Arc<RwLock<EventListeners>>,
    next_id: AtomicU64,
}

impl CDPInner {
    pub async fn connect(url: &str) -> Result<Arc<Self>, CdpError> {
        info!("Connecting to CDP at {}", url);
        let (ws, _) = connect_async(url).await?;
        info!("Connected to CDP successfully");

        let (tx, rx) = mpsc::unbounded_channel();

        let inner = Arc::new(Self {
            tx,
            pending: RwLock::new(HashMap::new()),
            event_listeners: Arc::new(RwLock::new(EventListeners::new())),
            next_id: AtomicU64::new(1),
        });

        // Start message loop
        let inner_clone = inner.clone();
        tokio::spawn(async move {
            inner_clone.message_loop(ws, rx).await;
        });

        Ok(inner)
    }

    pub async fn send_command<C: Command>(
        &self,
        cmd: C,
        session_id: Option<&str>,
    ) -> Result<C::Response, CdpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.pending.write().await.insert(id, tx);

        let mut msg = serde_json::json!({
            "id": id,
            "method": C::METHOD,
            "params": cmd,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = serde_json::json!(sid);
        }

        debug!(
            id = id,
            method = C::METHOD,
            session_id = session_id,
            "Sending command"
        );
        trace!("Command payload: {}", msg);

        self.tx
            .send(Message::Text(msg.to_string().into()))
            .map_err(|_| CdpError::ConnectionClosed)?;

        let response = rx.await.map_err(|_| CdpError::ChannelClosed)??;

        debug!(id = id, method = C::METHOD, "Received response");
        trace!("Response payload: {}", response);

        // Handle empty response for unit type
        if response.is_null() {
            serde_json::from_value(Value::Null).map_err(Into::into)
        } else if let Some(obj) = response.as_object() {
            if obj.is_empty() {
                serde_json::from_value(Value::Null).map_err(Into::into)
            } else {
                serde_json::from_value(response).map_err(Into::into)
            }
        } else {
            serde_json::from_value(response).map_err(Into::into)
        }
    }

    pub fn event_stream<T>(&self, event_name: &str) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_name = event_name.to_string();
        let listeners = Arc::clone(&self.event_listeners);

        debug!(event = %event_name, "Subscribing to event");

        let event_name_for_spawn = event_name.clone();
        tokio::spawn(async move {
            listeners
                .write()
                .await
                .add_listener(&event_name_for_spawn, tx);
        });

        Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx).filter_map(move |v| {
                let event_name = event_name.clone();
                async move {
                    match serde_json::from_value((*v).clone()) {
                        Ok(event) => Some(event),
                        Err(e) => {
                            warn!(event = %event_name, error = %e, "Failed to deserialize event");
                            trace!("Event data: {}", v);
                            None
                        }
                    }
                }
            }),
        )
    }

    async fn message_loop(&self, mut ws: WsStream, mut rx: mpsc::UnboundedReceiver<Message>) {
        debug!("Starting message loop");
        loop {
            tokio::select! {
                // Receive from WebSocket
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
                // Send to WebSocket
                Some(msg) = rx.recv() => {
                    if let Err(e) = ws.send(msg).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        }
        debug!("Message loop ended");
    }

    async fn handle_message(&self, value: Value) {
        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
            // Command response
            if let Some(tx) = self.pending.write().await.remove(&id) {
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
            // Event
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            trace!(event = method, "Dispatching event");
            self.event_listeners
                .write()
                .await
                .dispatch(method, Arc::new(params));
        } else {
            warn!("Received message without id or method: {}", value);
        }
    }
}
