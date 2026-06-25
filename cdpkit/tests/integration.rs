use cdpkit::{CdpError, Method, Sender, CDP};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

async fn start_mock_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (mut write, mut read) = ws.split();

                use futures::SinkExt;
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            if let Some(id) = val.get("id").and_then(|v| v.as_u64()) {
                                let method =
                                    val.get("method").and_then(|v| v.as_str()).unwrap_or("");

                                let result = match method {
                                    "Test.echo" => {
                                        val.get("params").cloned().unwrap_or(Value::Null)
                                    }
                                    "Test.error" => {
                                        let resp = json!({
                                            "id": id,
                                            "error": {"code": -32000, "message": "test error"}
                                        });
                                        let _ = write
                                            .send(Message::Text(resp.to_string().into()))
                                            .await;
                                        continue;
                                    }
                                    _ => json!({}),
                                };

                                let resp = json!({"id": id, "result": result});
                                let _ = write.send(Message::Text(resp.to_string().into())).await;
                            }
                        }
                    } else if let Message::Close(_) = msg {
                        break;
                    }
                }
            });
        }
    });

    (addr, handle)
}

#[derive(Debug, Clone, Serialize)]
struct TestEcho {
    value: String,
}

impl Method for TestEcho {
    type Response = TestEchoResponse;
    const METHOD: &'static str = "Test.echo";
}

impl TestEcho {
    async fn send(self, target: &(impl Sender + Sync)) -> Result<TestEchoResponse, CdpError> {
        target.send_cmd(self).await
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct TestEchoResponse {
    value: String,
}

#[derive(Debug, Serialize)]
struct TestError {}

impl Method for TestError {
    type Response = Value;
    const METHOD: &'static str = "Test.error";
}

impl TestError {
    async fn send(self, target: &(impl Sender + Sync)) -> Result<Value, CdpError> {
        target.send_cmd(self).await
    }
}

#[tokio::test]
async fn connect_and_send_command() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());

    let cdp = CDP::connect_ws(&url).await.unwrap();
    let resp = TestEcho {
        value: "hello".into(),
    }
    .send(&cdp)
    .await
    .unwrap();

    assert_eq!(
        resp,
        TestEchoResponse {
            value: "hello".into()
        }
    );
}

#[tokio::test]
async fn send_raw_command() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());

    let cdp = CDP::connect_ws(&url).await.unwrap();
    let resp = cdp
        .send_raw("Test.echo", json!({"value": "raw"}))
        .await
        .unwrap();

    assert_eq!(resp.get("value").and_then(|v| v.as_str()), Some("raw"));
}

#[tokio::test]
async fn protocol_error_returned() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());

    let cdp = CDP::connect_ws(&url).await.unwrap();
    let err = TestError {}.send(&cdp).await.unwrap_err();

    match err {
        CdpError::Protocol { code, message } => {
            assert_eq!(code, -32000);
            assert_eq!(message, "test error");
        }
        other => panic!("Expected Protocol error, got: {:?}", other),
    }
}

#[tokio::test]
async fn close_marks_connection_closed() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());

    let cdp = CDP::connect_ws(&url).await.unwrap();
    assert!(!cdp.is_closed());

    cdp.close().await;
    assert!(cdp.is_closed());

    let err = TestEcho {
        value: "fail".into(),
    }
    .send(&cdp)
    .await
    .unwrap_err();
    assert!(matches!(err, CdpError::ConnectionClosed));
}

#[tokio::test]
async fn clone_shares_connection() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());

    let cdp = CDP::connect_ws(&url).await.unwrap();
    let cdp2 = cdp.clone();

    let resp = TestEcho {
        value: "cloned".into(),
    }
    .send(&cdp2)
    .await
    .unwrap();
    assert_eq!(
        resp,
        TestEchoResponse {
            value: "cloned".into()
        }
    );

    cdp.close().await;
    assert!(cdp2.is_closed());
}

#[tokio::test]
async fn event_stream_receives_events() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        // Wait for any message then push an event
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let val: Value = serde_json::from_str(&text).unwrap();
            let id = val.get("id").and_then(|v| v.as_u64()).unwrap();

            let event = json!({
                "method": "Test.event",
                "params": {"data": "event_payload"}
            });
            let _ = write.send(Message::Text(event.to_string().into())).await;

            // Respond to the command
            let resp = json!({"id": id, "result": {}});
            let _ = write.send(Message::Text(resp.to_string().into())).await;
        }
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();

    #[derive(Debug, Deserialize)]
    struct TestEvent {
        data: String,
    }

    let mut events = cdp.event_stream::<TestEvent>("Test.event");

    // Send a command to trigger the server to push an event
    let _ = cdp.send_raw("Trigger.event", json!({})).await;

    let event = tokio::time::timeout(Duration::from_secs(2), events.next())
        .await
        .expect("timeout waiting for event")
        .expect("stream ended");

    assert_eq!(event.data, "event_payload");
}

#[tokio::test]
async fn connection_drop_drains_pending() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        // Accept connection, read one message, then close without responding
        let _ = read.next().await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = write.send(Message::Close(None)).await;
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();

    let err = tokio::time::timeout(
        Duration::from_secs(3),
        TestEcho {
            value: "pending".into(),
        }
        .send(&cdp),
    )
    .await
    .expect("should not timeout")
    .unwrap_err();

    assert!(matches!(err, CdpError::ConnectionClosed));
}

#[tokio::test]
async fn session_wrapper_sends_with_session_id() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    let id = val.get("id").and_then(|v| v.as_u64()).unwrap();
                    let session_id = val.get("sessionId").and_then(|v| v.as_str()).unwrap_or("");
                    let resp = json!({"id": id, "result": {"session": session_id}});
                    let _ = write.send(Message::Text(resp.to_string().into())).await;
                }
            }
        }
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();
    let session = cdp.session("my-session-123");

    let resp = session.send_raw("Test.echo", json!({})).await.unwrap();

    assert_eq!(
        resp.get("session").and_then(|v| v.as_str()),
        Some("my-session-123")
    );
}

#[tokio::test]
async fn concurrent_commands() {
    let (addr, _server) = start_mock_server().await;
    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let cdp = cdp.clone();
        handles.push(tokio::spawn(async move {
            let resp = TestEcho {
                value: format!("msg-{}", i),
            }
            .send(&cdp)
            .await
            .unwrap();
            resp
        }));
    }

    let mut results: Vec<String> = Vec::new();
    for h in handles {
        let resp = h.await.unwrap();
        results.push(resp.value);
    }
    results.sort();

    let mut expected: Vec<String> = (0..10).map(|i| format!("msg-{}", i)).collect();
    expected.sort();
    assert_eq!(results, expected);
}

#[tokio::test]
async fn event_stream_skips_bad_deserialization() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let val: Value = serde_json::from_str(&text).unwrap();
            let id = val.get("id").and_then(|v| v.as_u64()).unwrap();

            // Send a malformed event (missing required field)
            let bad_event = json!({"method": "Test.event", "params": {"wrong_field": 123}});
            let _ = write
                .send(Message::Text(bad_event.to_string().into()))
                .await;

            // Send a valid event
            let good_event = json!({"method": "Test.event", "params": {"data": "valid"}});
            let _ = write
                .send(Message::Text(good_event.to_string().into()))
                .await;

            // Respond to the command
            let resp = json!({"id": id, "result": {}});
            let _ = write.send(Message::Text(resp.to_string().into())).await;
        }
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();

    #[derive(Debug, Deserialize)]
    struct TestEvent {
        data: String,
    }

    let mut events = cdp.event_stream::<TestEvent>("Test.event");

    let _ = cdp.send_raw("Trigger", json!({})).await;

    // Should skip the bad event and receive the good one
    let event = tokio::time::timeout(Duration::from_secs(2), events.next())
        .await
        .expect("timeout")
        .expect("stream ended");

    assert_eq!(event.data, "valid");
}

#[tokio::test]
async fn discover_ws_url_integration() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Mock HTTP server that returns /json/version
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let _ = stream.read(&mut buf).await.unwrap();

        let ws_url = format!("ws://127.0.0.1:{}/devtools/browser/fake-id", port);
        let body = json!({"webSocketDebuggerUrl": ws_url}).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });

    // Mock WebSocket server on the same port won't work, so just test that
    // connect() correctly parses the HTTP response and attempts WebSocket connection.
    // The WebSocket connection will fail, but we verify the HTTP parsing works.
    let result = CDP::connect(&format!("localhost:{}", port)).await;

    // Should fail at WebSocket stage (not HTTP parsing stage)
    match result {
        Err(CdpError::WebSocket(_)) => {} // expected: HTTP worked, WS handshake failed
        Err(CdpError::ConnectionFailed(msg)) => {
            // Also acceptable if connection is refused on WS attempt
            assert!(
                !msg.contains("No Content-Length") && !msg.contains("No webSocketDebuggerUrl"),
                "HTTP parsing failed unexpectedly: {}",
                msg
            );
        }
        Ok(_) => panic!("Should not succeed without a real WebSocket server"),
        Err(e) => panic!("Unexpected error type: {:?}", e),
    }
}

#[tokio::test]
async fn owned_session_across_spawn() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    let id = val.get("id").and_then(|v| v.as_u64()).unwrap();
                    let session_id = val.get("sessionId").and_then(|v| v.as_str()).unwrap_or("");
                    let resp = json!({"id": id, "result": {"session": session_id}});
                    let _ = write.send(Message::Text(resp.to_string().into())).await;
                }
            }
        }
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();
    let owned = cdp.owned_session("owned-123");

    let handle = tokio::spawn(async move { owned.send_raw("Test.echo", json!({})).await.unwrap() });

    let resp = handle.await.unwrap();
    assert_eq!(
        resp.get("session").and_then(|v| v.as_str()),
        Some("owned-123")
    );
}

#[tokio::test]
async fn session_event_stream_filters_by_session_id() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();

        use futures::SinkExt;
        if let Some(Ok(Message::Text(text))) = read.next().await {
            let val: Value = serde_json::from_str(&text).unwrap();
            let id = val.get("id").and_then(|v| v.as_u64()).unwrap();

            let event_a = json!({
                "method": "Test.event",
                "sessionId": "session-A",
                "params": {"data": "for-A"}
            });
            let _ = write.send(Message::Text(event_a.to_string().into())).await;

            let event_b = json!({
                "method": "Test.event",
                "sessionId": "session-B",
                "params": {"data": "for-B"}
            });
            let _ = write.send(Message::Text(event_b.to_string().into())).await;

            let event_a2 = json!({
                "method": "Test.event",
                "sessionId": "session-A",
                "params": {"data": "for-A-again"}
            });
            let _ = write.send(Message::Text(event_a2.to_string().into())).await;

            let resp = json!({"id": id, "result": {}});
            let _ = write.send(Message::Text(resp.to_string().into())).await;
        }
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();
    let session_a = cdp.session("session-A");

    #[derive(Debug, Deserialize)]
    struct SessionEvent {
        data: String,
    }

    let mut events = session_a.event_stream::<SessionEvent>("Test.event");

    let _ = cdp.send_raw("Trigger", json!({})).await;

    let event1 = tokio::time::timeout(Duration::from_secs(2), events.next())
        .await
        .expect("timeout")
        .expect("stream ended");
    assert_eq!(event1.data, "for-A");

    let event2 = tokio::time::timeout(Duration::from_secs(2), events.next())
        .await
        .expect("timeout")
        .expect("stream ended");
    assert_eq!(event2.data, "for-A-again");
}

#[tokio::test]
async fn connect_timeout_triggers() {
    // Bind a TCP listener but never perform WebSocket upgrade — the client
    // should time out waiting for the handshake to complete.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Accept TCP but do nothing (no WS upgrade, no data)
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        // Hold the connection open without sending anything
        tokio::time::sleep(Duration::from_secs(60)).await;
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let result = CDP::connect_ws_with_timeout(&url, Duration::from_millis(500)).await;

    match result {
        Err(CdpError::ConnectionFailed(msg)) => {
            assert!(
                msg.contains("timed out"),
                "Expected 'timed out' in error message, got: {}",
                msg
            );
        }
        Ok(_) => panic!("Expected connection to time out, but it succeeded"),
        Err(e) => panic!("Expected ConnectionFailed with timeout message, got: {:?}", e),
    }
}

#[tokio::test]
async fn command_timeout_triggers() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server accepts connection but never responds to commands
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (_write, mut read) = ws.split();
        // Read messages but never respond
        while let Some(Ok(_)) = read.next().await {}
    });

    let url = format!("ws://127.0.0.1:{}", addr.port());
    let cdp = CDP::connect_ws(&url).await.unwrap();

    // Set a very short timeout
    cdp.set_command_timeout(Duration::from_millis(100));

    let err = TestEcho {
        value: "timeout-test".into(),
    }
    .send(&cdp)
    .await
    .unwrap_err();

    assert!(matches!(err, CdpError::Timeout));
}
