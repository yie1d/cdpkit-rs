[English](../README.md) | **[中文](README.zh.md)**

类型安全的 Rust [Chrome DevTools Protocol (CDP)](https://chromedevtools.github.io/devtools-protocol/) 客户端，支持 async/await。

纯 CDP 协议实现，不是另一个浏览器自动化库。

## 特性

- 🔒 **类型安全** - 所有 CDP 命令和事件都是强类型的，编译时检查参数和返回值
- 🚀 **异步优先** - 基于 tokio 构建，完整的 async/await 支持
- 📡 **流式事件** - 使用 Rust Stream 处理 CDP 事件，支持多路复用和过滤
- 🎯 **纯协议客户端** - 直接访问 CDP，无封装层，完全控制
- 🔄 **自动生成绑定** - 从仓库内固定的官方 CDP 快照可复现生成
- 🪶 **轻量级** - 最小化依赖，专注于协议通信
- 🔌 **灵活连接** - 连接到已运行的浏览器实例，无需管理进程
- 🧩 **动态命令** - 不需要类型绑定时，可按方法名发送任意 CDP 命令

## 前置条件

启动 Chrome 并开启远程调试端口：

```bash
# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile

# Linux
google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile

# Windows
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222 --user-data-dir=%TEMP%\cdp-profile

# 无头模式（无 UI）
chrome --headless --remote-debugging-port=9222 --user-data-dir=/tmp/cdp-profile
```

> **注意：** 如果 Chrome 已经在运行，`--remote-debugging-port` 参数会被忽略（Chrome 会合并到已有实例）。使用 `--user-data-dir` 可以强制启动独立实例，或者先关闭所有 Chrome 窗口/进程再启动。

> **想直接控制已登录的 Chrome？** Chrome 136+ 支持通过一个开关复用真实 profile，无需新起实例。详见 **[接管已登录的 Chrome](connect-existing-chrome.zh.md)**。

## 核心概念

- **CDP** — 浏览器级别连接。用于浏览器命令（创建/关闭标签页）。
- **Session** — 页面级别会话，绑定到特定标签页。用于页面命令（导航、DOM、网络）。
- **OwnedSession** — 与 `Session` 类似但拥有连接（`Send + 'static`）。需要在 `tokio::spawn` 中使用时，用 `cdp.owned_session(id)` 创建。
- **Sender trait** — `CDP` 和 `Session` 都实现了 `Sender`。传 `&cdp` 执行浏览器命令，传 `&session` 执行页面命令。
- **Enable** — CDP 要求先启用域（如 `page::methods::Enable`）才能接收该域的事件。
- **flatten 模式** — `AttachToTarget` / `SetAutoAttach` 默认 `flatten: true`（session 事件正常工作的前提）。现在显式传 `with_flatten(false)` 会直接返回 `CdpError::UnsupportedConfiguration(...)`，不再静默创建本库不支持的 non-flatten session。
- **事件订阅顺序** — 必须在触发动作之前订阅事件，否则事件可能丢失。
- **事件通道** — `event_stream()` / 生成的 `Event::subscribe()` 默认保持无界并在日志后跳过反序列化失败的事件。bounded result stream 可通过 `EventStreamStats::dropped_events()` 查询 `DropNewest` 丢弃数；`CloseStream` 会返回一次 `CdpError::EventStreamOverflow` 后结束。
- **连接输入** — `CDP::connect(...)` 只接受 `host:port` 或 `http://host:port` 并固定请求 `/json/version`。完整的 `ws://` 或 `wss://` DevTools URL 必须传给 `CDP::connect_ws(...)`。
- **连接错误** — `CdpError` 针对每个失败阶段有专用变体：`Io`、`DiscoveryTimeout`、`HandshakeTimeout`、`HttpStatus`、`InvalidDiscoveryInput`、`InvalidDiscoveryResponse`。可用 `err.is_connection_failed()` 或 `err.is_timeout()` 做宽泛判断。
- **CloseReason** — `CDP::close_reason()` 返回连接关闭原因（`Normal` / `Remote` / `Error`）。所有 `CDP` handle drop 后连接会自动关闭。
- **关闭等待** — `CDP::closed().await` 会在后台消息循环真正完成 WebSocket shutdown 后才返回。

## 快速开始

```rust
use cdpkit::{CDP, page, target};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 Chrome
    let cdp = CDP::connect("localhost:9222").await?;

    // 创建新页面（浏览器级别命令 → 传 &cdp）
    let result = target::methods::CreateTarget::new("https://example.com")
        .send(&cdp)
        .await?;

    // 附加到页面
    let attach = target::methods::AttachToTarget::new(result.target_id)
        // flatten 模式为默认值（session 事件正常工作的前提）
        .send(&cdp)
        .await?;

    // 创建 session 用于页面级别命令
    let session = cdp.session(attach.session_id);

    // 启用页面域（页面级别 → 传 &session）
    page::methods::Enable::new().send(&session).await?;

    // 先订阅事件，再触发产生事件的操作
    let mut events = page::events::LoadEventFired::subscribe(&session);

    page::methods::Navigate::new("https://rust-lang.org")
        .send(&session)
        .await?;

    // 等待事件
    if let Some(event) = events.next().await {
        println!("页面加载完成，时间戳: {}", event.timestamp);
    }

    Ok(())
}
```

## 安装

```toml
[dependencies]
cdpkit = "0.5"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
futures = "0.3"
```

## 文档

- **[示例代码](../cdpkit/examples/)** - 可运行的代码示例
- **[接管已登录的 Chrome](connect-existing-chrome.zh.md)** - 复用已登录 Chrome（Chrome 136+）

## 代码生成

cdpkit 使用从官方 Chrome DevTools Protocol 规范自动生成的绑定。`cdpkit_codegen` 工具负责处理此过程。

### 重新生成 CDP 绑定

从仓库内固定的协议快照重新生成 CDP 绑定：

```bash
# 运行代码生成器
cargo run -p cdpkit_codegen

# 生成的代码将写入 cdpkit/src/protocol.rs
```

只有在明确更新协议快照时才运行 `cargo run -p cdpkit_codegen -- --update` 联网获取官方版本；应同时审查并提交两份 JSON 输入和生成结果。默认生成与 CI 不依赖实时网络。

### 工作原理

1. **读取协议** - 读取已提交的 `browser_protocol.json` 与 `js_protocol.json`（`--update` 才联网刷新）
2. **解析规范** - 将协议定义解析为 Rust 数据结构
3. **生成代码** - 为所有 CDP 域、命令和事件生成类型安全的 Rust 代码
4. **输出** - 将生成的代码写入 `cdpkit/src/protocol.rs`

生成的代码包括：
- 所有 CDP 域（Page、Network、Runtime 等）
- 带有构建器模式的强类型命令结构，位于 `methods` 子模块
- 响应类型位于 `responses` 子模块，用于方法返回值
- 事件类型位于 `events` 子模块，用于订阅
- 类型定义位于 `types` 子模块，用于参数和共享类型

### 何时重新生成

- 当 Chrome 发布新的 CDP 版本时
- 当你需要实验性 CDP 功能时
- 当贡献协议绑定更新时

**注意：** 协议 JSON 与生成的 `protocol.rs` 均提交到版本控制。CI 使用固定输入离线生成，并验证连续生成保持字节级稳定。

## 为什么选择 cdpkit？

### 直接的协议访问

cdpkit 提供对 CDP 的直接访问，让你完全控制浏览器行为：

```rust
// 直接发送 CDP 命令，支持所有参数
let session = cdp.session(session_id);
page::methods::Navigate::new("https://example.com")
    .with_referrer("https://google.com")
    .with_transition_type(page::types::TransitionType::Link)
    .send(&session)
    .await?;

// 使用 FromStr 从字符串解析枚举值
let transition: page::types::TransitionType = "link".parse()
    .expect("invalid transition type");

// 使用 AsRef<str> 将枚举值转换为字符串
let s: &str = page::types::TransitionType::Link.as_ref(); // "link"

// 访问完整的返回数据
let result = runtime::methods::Evaluate::new("document.title")
    .with_return_by_value(true)
    .send(&session)
    .await?;

// 不需要类型绑定时，按方法名动态发送命令
let result = session.send_raw(
    "Page.navigate",
    serde_json::json!({"url": "https://example.com"}),
).await?;
println!("Frame ID: {}", result["frameId"]);
```

### 强大的事件处理

基于 Rust Stream 的事件系统，支持组合、过滤和多路复用：

```rust
use cdpkit::{EventOverflowStrategy, EventStreamPolicy};
use futures::StreamExt;
use std::num::NonZeroUsize;

// 订阅类型化事件（按 session 过滤）
let mut load_events = page::events::LoadEventFired::subscribe(&session);
let mut nav_events = page::events::FrameNavigated::subscribe(&session);

// 使用 tokio::select! 处理多个事件流
loop {
    tokio::select! {
        Some(event) = load_events.next() => {
            println!("页面加载: {}", event.timestamp);
        }
        Some(event) = nav_events.next() => {
            println!("导航到: {:?}", event.frame.url);
        }
        else => break,
    }
}

// 或按事件名动态订阅
let mut requests = cdp.event_stream::<serde_json::Value>("Network.requestWillBeSent");
while let Some(event) = requests.next().await {
    println!("请求: {}", event["request"]["url"]);
}

let mut request_events = network::events::RequestWillBeSent::subscribe_result_with_policy(
    &session,
    EventStreamPolicy::Bounded {
        capacity: NonZeroUsize::new(256).unwrap(),
        overflow: EventOverflowStrategy::DropNewest,
    },
);
let request_stats = request_events.stats();

while let Some(event) = request_events.next().await {
    match event {
        Ok(event) => println!("请求: {}", event.request.url),
        Err(err) => eprintln!("事件反序列化失败: {err}"),
    }
}
println!("已丢弃事件数：{}", request_stats.dropped_events());
```

### 编译时类型安全

所有 CDP 操作都经过类型检查，在编译时捕获错误：

```rust
// ✅ 类型检查：必需参数
let cmd = page::methods::Navigate::new("https://example.com");

// ✅ 类型检查：返回值
let result = cmd.send(&session).await?;
let frame_id: String = result.frame_id;  // 类型已知

// ❌ 编译错误：缺少参数
let cmd = page::methods::Navigate::new();  // 错误：缺少 url

// ❌ 编译错误：类型不匹配
let cmd = page::methods::Navigate::new(123);  // 错误：期望 String
```

### 适用场景

cdpkit 适合以下场景：

- **自动化测试** - 需要精确控制浏览器行为
- **网页爬虫** - 需要监控网络请求和响应
- **性能分析** - 需要访问详细的性能指标
- **调试工具** - 构建自定义的开发者工具
- **浏览器扩展** - 需要底层 CDP 访问

## 许可证

采用以下任一许可证：

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

由您选择。

## 参考
- [cdp-use](https://github.com/browser-use/cdp-use)
- [chromiumoxide](https://github.com/mattsse/chromiumoxide)
