[English](../README.md) | **[中文](README.zh.md)**

类型安全的 Rust [Chrome DevTools Protocol (CDP)](https://chromedevtools.github.io/devtools-protocol/) 客户端，支持 async/await。

## 特性

- 🔒 **类型安全** - 所有 CDP 命令和事件都是强类型的，编译时检查参数和返回值
- 🚀 **异步优先** - 基于 tokio 构建，完整的 async/await 支持
- 📡 **流式事件** - 使用 Rust Stream 处理 CDP 事件，支持多路复用和过滤
- 🎯 **纯协议客户端** - 直接访问 CDP，无封装层，完全控制
- 🔄 **自动生成绑定** - 从官方 CDP 规范自动生成，始终保持最新
- 🪶 **轻量级** - 最小化依赖，专注于协议通信
- 🔌 **灵活连接** - 连接到已运行的浏览器实例，无需管理进程

## 快速开始

```rust
use cdpkit::{CDP, Method, page, target};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 Chrome
    let cdp = CDP::connect("localhost:9222").await?;
    
    // 创建新页面
    let result = target::methods::CreateTarget::new("https://example.com")
        .send(&cdp, None)
        .await?;
    
    // 附加到页面
    let attach = target::methods::AttachToTarget::new(result.target_id)
        .with_flatten(true)
        .send(&cdp, None)
        .await?;
    
    let session = attach.session_id;
    
    // 导航并监听事件
    page::methods::Enable::new().send(&cdp, Some(&session)).await?;
    page::methods::Navigate::new("https://rust-lang.org")
        .send(&cdp, Some(&session))
        .await?;
    
    let mut events = page::events::LoadEventFired::subscribe(&cdp);
    if let Some(event) = events.next().await {
        println!("页面加载完成，时间戳: {}", event.timestamp);
    }
    
    Ok(())
}
```

## 安装

```toml
[dependencies]
cdpkit = "0.2"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

## 文档

- **[示例代码](../cdpkit/examples/)** - 可运行的代码示例

## 代码生成

cdpkit 使用从官方 Chrome DevTools Protocol 规范自动生成的绑定。`cdpkit_codegen` 工具负责处理此过程。

### 重新生成 CDP 绑定

要将 CDP 绑定更新到最新的协议版本：

```bash
# 运行代码生成器
cargo run -p cdpkit_codegen

# 生成的代码将写入 cdpkit/src/protocol.rs
```

### 工作原理

1. **获取协议** - 从 Chrome 仓库下载最新的 CDP 协议 JSON
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

**注意：** 生成的 `protocol.rs` 文件已提交到版本控制，因此用户无需运行生成器，除非想要更新协议版本。

## 为什么选择 cdpkit？

### 直接的协议访问

cdpkit 提供对 CDP 的直接访问，让你完全控制浏览器行为：

```rust
// 直接发送 CDP 命令，支持所有参数
page::methods::Navigate::new("https://example.com")
    .with_referrer("https://google.com")
    .with_transition_type(page::types::TransitionType::Link)
    .send(&cdp, Some(&session))
    .await?;

// 使用 FromStr 从字符串解析枚举值
let transition: page::types::TransitionType = "link".parse()
    .expect("invalid transition type");

// 使用 AsRef<str> 将枚举值转换为字符串
let s: &str = page::types::TransitionType::Link.as_ref(); // "link"

// 访问完整的返回数据
let result = runtime::methods::Evaluate::new("document.title")
    .with_return_by_value(true)
    .send(&cdp, Some(&session))
    .await?;
```

### 强大的事件处理

基于 Rust Stream 的事件系统，支持组合、过滤和多路复用：

```rust
use futures::StreamExt;

// 订阅多个事件
let mut load_events = page::events::LoadEventFired::subscribe(&cdp);
let mut console_events = runtime::events::ConsoleAPICalled::subscribe(&cdp);

// 使用 Stream 组合器
let mut combined = futures::stream::select(load_events, console_events);

// 过滤和处理
while let Some(event) = combined.next().await {
    // 处理事件
}
```

### 编译时类型安全

所有 CDP 操作都经过类型检查，在编译时捕获错误：

```rust
// ✅ 类型检查：必需参数
let cmd = page::methods::Navigate::new("https://example.com");

// ✅ 类型检查：返回值
let result = cmd.send(&cdp, Some(&session)).await?;
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
