# 接管已登录的 Chrome（Chrome 136+）

连接到你正在使用的 Chrome，保留所有 cookie、登录态和扩展。这与传统的 `--remote-debugging-port` 新实例方式互补。

## 前置条件

- Chrome 136 或更高版本
- 开启远程调试开关：打开 `chrome://inspect/#remote-debugging`，启用 **"Allow remote debugging for this browser instance"**

该设置写入 profile 的 `Local State` 文件，重启浏览器后仍然生效。

## 为什么不能用 `CDP::connect("localhost:port")`？

通过开关启用调试（而非 `--remote-debugging-port`）时，HTTP 发现端点 `/json/version` 被禁用（返回 404）。`CDP::connect("host:port")` 依赖该端点获取 WebSocket URL，因此会失败。

正确做法是读取 `DevToolsActivePort` 文件，直接用 WebSocket URL 连接。

## 读取 DevToolsActivePort

Chrome 启动时会在用户数据目录写入 `DevToolsActivePort` 文件，内容为两行：

```
<端口>
<ws路径>
```

例如：
```
9222
/devtools/browser/b0e3c5a1-...
```

端口每次启动可能不同，切勿硬编码。

### 各平台文件路径

| 平台    | 路径 |
|---------|------|
| Windows | `%LOCALAPPDATA%\Google\Chrome\User Data\DevToolsActivePort` |
| macOS   | `~/Library/Application Support/Google/Chrome/DevToolsActivePort` |
| Linux   | `~/.config/google-chrome/DevToolsActivePort` |

> 非默认 profile 或其他 Chromium 内核浏览器的用户数据目录可能不同，请根据实际情况调整。

## 使用 cdpkit 连接

```rust
use cdpkit::CDP;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 读取 DevToolsActivePort（请根据你的操作系统调整路径）
    let path = r"C:\Users\<you>\AppData\Local\Google\Chrome\User Data\DevToolsActivePort";
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();
    let port = lines.next().unwrap().trim();
    let ws_path = lines.next().unwrap_or("").trim();

    // 直接 WebSocket 连接（绕过被禁用的 /json/version）
    let ws_url = format!("ws://127.0.0.1:{}{}", port, ws_path);
    let cdp = CDP::connect_with_timeout(&ws_url, Duration::from_secs(10)).await?;

    println!("已连接到现有 Chrome！");
    Ok(())
}
```

## 注意事项

- 连接的是用户默认的 BrowserContext——cookie、登录态、扩展全部共享。
- `Target::createTarget` 不传 `browserContextId` 时，会在用户可见的浏览器窗口中打开新标签页。
- DevTools 前端本身也是一个 CDP client，多个 client 可以同时连接同一个浏览器，没有排他锁。
- 建议使用 `CDP::connect_with_timeout` 而非普通 `connect`，避免 `DevToolsActivePort` 文件过期（指向已退出的 Chrome 进程端口）时无限挂起。
