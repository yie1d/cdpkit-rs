# Changelog

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
