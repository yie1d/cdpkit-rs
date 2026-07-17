# Changelog

## [0.5.0] - 2026-07-17

### Breaking Changes

- `CDP::connect(...)` and `CDP::connect_with_timeout(...)` now accept only `host:port` or `http://host:port` discovery inputs and always request `/json/version`. Direct `ws://` / `wss://` URLs must use `CDP::connect_ws(...)` or `CDP::connect_ws_with_timeout(...)`.
- `Target.attachToTarget` / `Target.setAutoAttach` reject `.with_flatten(false)` with `CdpError::UnsupportedConfiguration(...)`; cdpkit supports flattened sessions only.
- `EventStreamResult<T>` is now a concrete `Stream` type rather than a boxed-stream type alias so callers can retain `EventStreamStats`. Normal `.next()` usage is unchanged, but code naming the old alias representation may need adjustment.
- Downstream browserkit must update its cdpkit dependency and migrate direct WebSocket calls to `connect_ws` / `connect_ws_with_timeout` before adopting 0.5.0. This release does not update browserkit automatically.

### Added

- `CDP::closed()` — async method that resolves only after the WebSocket loop, close reason, pending commands, and event listeners finish shutdown; late and concurrent waiters are supported.
- `EventStreamResult<T>`, `EventStreamStats`, `EventOverflowStrategy`, and `EventStreamPolicy` — event-stream types for explicit buffering, errors, and overflow telemetry.
- `Sender::event_stream_with_policy()` / generated `Event::subscribe_with_policy()` — opt-in bounded event buffering with explicit overflow handling while leaving existing unbounded subscriptions unchanged.
- `Sender::event_stream_result()` and `Sender::event_stream_result_with_policy()` plus generated `Event::subscribe_result()` helpers — APIs that surface deserialization failures and terminal `CdpError::EventStreamOverflow` errors.
- `EventStreamStats::dropped_events()` reports every event rejected by `DropNewest`; `CloseStream` emits one structured overflow error after buffered events and then ends.
- Complete byte-for-byte codegen golden coverage for the fixed mini protocol fixture.

### Fixed

- `CDP::closed()` completion is durable when no waiter exists at shutdown and is published only after all shutdown state is finalized.
- The two official protocol JSON inputs are checked in, so default code generation and CI do not depend on live network access.
- The locked workspace dependency graph builds with the declared Rust 1.75 MSRV and no longer resolves edition-2024-only `cpufeatures`.

### Changed

- `cargo run -p cdpkit_codegen` regenerates from the committed protocol snapshot. Use `cargo run -p cdpkit_codegen -- --update` only when intentionally refreshing that snapshot from the official source.
- CI and release use the same fmt/build/test/clippy/examples/MSRV/codegen/package gates; release inputs are validated before shell use and third-party actions are pinned to immutable commits.

## [0.4.0] - 2026-06-26

### Breaking Changes

- `CdpError::ConnectionFailed(String)` removed — replaced by specific variants:
  - `CdpError::Io(String)` — TCP connect / I/O errors
  - `CdpError::DiscoveryTimeout` — HTTP `/json/version` discovery timed out
  - `CdpError::HandshakeTimeout` — WebSocket handshake timed out
  - `CdpError::HttpStatus(u16)` — Chrome returned non-200 HTTP status
  - `CdpError::InvalidDiscoveryResponse(String)` — malformed discovery response
- `CdpError` is now `#[non_exhaustive]` — `match` arms require a `_` wildcard
- `CloseReason` is now `#[non_exhaustive]`
- `AttachToTarget::new()` and `SetAutoAttach::new()` now default `flatten` to `Some(true)` — previously `None` (non-flatten mode, which cdpkit does not support)

### Added

- `CloseReason` enum (`Normal` / `Remote` / `Error(String)`) and `CDP::close_reason()` — inspect why a connection ended
- `CdpError::is_timeout()` — returns `true` for `Timeout`, `DiscoveryTimeout`, `HandshakeTimeout`
- `CdpError::is_connection_failed()` — returns `true` for all connection-phase errors

### Migration from 0.3.x

```rust
// Before
match err {
    CdpError::ConnectionFailed(msg) if msg.contains("timed out") => { /* retry */ }
    CdpError::ConnectionFailed(_) => { /* config error */ }
    _ => {}
}

// After
match err {
    e if e.is_timeout() => { /* retry */ }
    CdpError::HttpStatus(code) => { /* Chrome returned HTTP {code} */ }
    CdpError::InvalidDiscoveryResponse(_) => { /* config error */ }
    CdpError::Io(_) => { /* network error */ }
    _ => {}
}
```

## [0.3.2] - 2026-06-26

### Changed
- Event channels are now unbounded — events are never silently dropped under backpressure. Previously a bounded channel of 1024 slots per subscription would drop events when full. Slow consumers should use `tokio::spawn` to process events off the stream loop to avoid unbounded memory growth.

## [0.3.1] - 2026-06-26

### Added
- `CDP::connect_with_timeout(target, Duration)` and `CDP::connect_ws_with_timeout(url, Duration)` — connect with explicit WebSocket handshake timeout
- `DEFAULT_CONNECT_TIMEOUT` constant (30s) — applied automatically by `CDP::connect` / `CDP::connect_ws`; previously those calls could hang indefinitely on unreachable endpoints
- Guide for connecting to an existing logged-in Chrome instance (Chrome 136+ toggle mode): `docs/connect-existing-chrome.md` / `docs/connect-existing-chrome.zh.md`

## [0.3.0] - 2026-05-08

### Breaking Changes

- **`send()` signature changed**: No longer accepts `session_id: Option<&str>`. Pass `&cdp` for browser-level or `&session` for page-level commands.
- **`subscribe()` signature changed**: Now accepts `&impl Sender` instead of `&CDP`.
- **Removed `CDP::send_raw(method, params, session_id)`**: Use `Sender::send_raw(method, params)` via `&cdp` or `&session` instead.
- **Removed `CDP::event_stream(name)`**: Use `Sender::event_stream(name)` via `&cdp` or `&session` instead.
- **New `Sender` trait**: Unified interface with `send_cmd`, `send_raw`, and `event_stream` methods.

### Migration from 0.2.x

```rust
// Before (0.2.x)
cmd.send(&cdp, Some(&session_id)).await?;
cmd.send(&cdp, None).await?;
Event::subscribe(&cdp);
cdp.send_raw("Method", params, Some(&session_id)).await?;

// After (0.3.0)
let session = cdp.session(session_id);
cmd.send(&session).await?;       // page-level
cmd.send(&cdp).await?;           // browser-level
Event::subscribe(&session);       // session-filtered
session.send_raw("Method", params).await?;  // via Sender trait
```

### Added
- `CDP::connect_with_timeout(target, Duration)` and `CDP::connect_ws_with_timeout(url, Duration)` — connect with explicit WebSocket handshake timeout
- `DEFAULT_CONNECT_TIMEOUT` constant (30s) — applied automatically by `CDP::connect` / `CDP::connect_ws`; previously those calls could hang indefinitely on unreachable endpoints
- `Sender` trait (sealed) — unified interface with `send_cmd`, `send_raw`, `event_stream`
- `EventStream<T>` type alias for event stream return types
- `Session<'a>` — borrowed session bound to a target
- `OwnedSession` — owned session for cross-task use (`Send + 'static`)
- `CDP::session()` and `CDP::owned_session()` constructors
- `CDP::close()` and `CDP::is_closed()` for graceful connection shutdown
- `Clone` implementation for `CDP`
- `CdpError::Timeout` variant for command timeout (30s default)
- `CDP::set_command_timeout()` for configurable timeout
- Session-aware event filtering — events are filtered by session_id
- `#[diagnostic::on_unimplemented]` for clear compile errors when passing wrong type
- `PartialEq`, `Eq`, `Hash` derives on all generated enums
- `Display` implementation on all generated enums (delegates to `AsRef<str>`)
- `Default` derive on commands with no required parameters
- Unit tests for `EventListeners` (dispatch, cleanup, session filtering)
- Integration tests with mock WebSocket server (14 tests)
- GitHub Actions CI (fmt, clippy, test on Linux/Windows/macOS, MSRV, docs, security audit)
- `deny.toml` for license and vulnerability checking
- `dependabot.yml` for automated dependency updates
- MSRV declared as Rust 1.75

### Changed
- `Sender` trait methods use natural names: `send_raw` (not `send_raw_cmd`), `event_stream` (not `subscribe_event`)
- `event_listeners` uses `std::sync::RwLock` instead of `tokio::sync::RwLock` (fixes race condition)
- `pending` uses `tokio::sync::Mutex` instead of `RwLock` (no read operations needed)
- `try_send` replaced with `.send().await` for proper backpressure
- Removed `reqwest` runtime dependency — HTTP discovery uses raw TCP
- `WS_SEND_CAPACITY` reduced from 2048 to 256
- Improved error messages for connection failures
- HTTP status code validation in `discover_ws_url`
- HTTP discovery has 10s timeout, Content-Length limit (1MB), header count limit (100)
- Message loop exits cleanly when all CDP handles are dropped
- Message loop clears event listeners on exit (prevents stream hang)
- WebSocket Close frame is properly replied to
- Generated command fields are now `pub` (consistent with response/event fields)
- Generated commands use `impl Into<String>` only for String parameters (not bool/i64/etc)
- `Method` trait now requires `Send` bound (removes need for `+ Send` at call sites)

### Fixed
- Race condition in `event_stream` where events could be lost before listener registration
- Pending commands now receive `ConnectionClosed` error when connection drops
- TOCTOU race between closed check and pending insertion
- Command timeout prevents indefinite hanging

## [0.2.2] - 2026-02-07

### Changed
- Bump version

## [0.2.1] - 2026-02-06

### Added
- `send_raw` and `event_stream` APIs

## [0.2.0] - 2026-02-06

### Changed
- Optimize dependencies and add convenience methods

## [0.1.0] - 2026-02-06

### Added
- Initial release with type-safe CDP bindings
