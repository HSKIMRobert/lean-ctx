use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::daemon;

/// Send an HTTP request to the daemon over the Unix Domain Socket.
/// Returns the response body as a string.
pub async fn daemon_request(method: &str, path: &str, body: &str) -> Result<String> {
    let socket_path = daemon::daemon_socket_path();
    if !socket_path.exists() {
        anyhow::bail!(
            "Daemon socket not found at {}. Is the daemon running?",
            socket_path.display()
        );
    }

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .with_context(|| format!("cannot connect to daemon at {}", socket_path.display()))?;

    let request = format_http_request(method, path, body);
    stream
        .write_all(request.as_bytes())
        .await
        .context("failed to write request to daemon socket")?;

    let mut response_buf = Vec::with_capacity(4096);
    stream
        .read_to_end(&mut response_buf)
        .await
        .context("failed to read response from daemon")?;

    parse_http_response(&response_buf)
}

/// Check if the daemon is reachable by hitting /health.
pub async fn daemon_health_check() -> bool {
    match daemon_request("GET", "/health", "").await {
        Ok(body) => body.trim() == "ok",
        Err(_) => false,
    }
}

/// Call a tool on the daemon's REST API.
pub async fn daemon_tool_call(name: &str, arguments: Option<&serde_json::Value>) -> Result<String> {
    let body = serde_json::json!({
        "name": name,
        "arguments": arguments,
    });
    daemon_request("POST", "/v1/tools/call", &body.to_string()).await
}

fn format_http_request(method: &str, path: &str, body: &str) -> String {
    if body.is_empty() {
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    } else {
        let content_length = body.len();
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n{body}"
        )
    }
}

fn parse_http_response(raw: &[u8]) -> Result<String> {
    let response_str = std::str::from_utf8(raw).context("daemon response is not valid UTF-8")?;

    let Some(header_end) = response_str.find("\r\n\r\n") else {
        anyhow::bail!("malformed HTTP response from daemon (no header boundary)");
    };

    let headers = &response_str[..header_end];
    let body = &response_str[header_end + 4..];

    let status_line = headers.lines().next().unwrap_or("");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    if status_code >= 400 {
        anyhow::bail!("daemon returned HTTP {status_code}: {body}");
    }

    Ok(body.to_string())
}

/// Attempt to connect to the daemon. Returns `None` if not running.
pub async fn try_daemon_request(method: &str, path: &str, body: &str) -> Option<String> {
    if !daemon::is_daemon_running() {
        return None;
    }
    daemon_request(method, path, body).await.ok()
}

/// Blocking helper for CLI commands: calls a daemon tool if the daemon is running.
/// Returns `None` if the daemon is not running or the call fails.
/// On Unix, attempts to auto-start the daemon if it's not already running.
#[allow(clippy::needless_pass_by_value)]
pub fn try_daemon_tool_call_blocking(
    name: &str,
    arguments: Option<serde_json::Value>,
) -> Option<String> {
    use std::time::Duration;

    // Always create the runtime once per CLI call. We also use it for
    // best-effort health checks while a daemon may be starting.
    let rt = tokio::runtime::Runtime::new().ok()?;

    let socket_path = daemon::daemon_socket_path();
    let mut ready = socket_path.exists() && rt.block_on(async { daemon_health_check().await });

    if !ready {
        #[cfg(unix)]
        {
            // Prevent double-daemon races when multiple CLI commands auto-start concurrently.
            // One process starts; others wait briefly for readiness.
            let lock = crate::core::startup_guard::try_acquire_lock(
                "daemon-start",
                Duration::from_millis(1200),
                Duration::from_secs(8),
            );

            if let Some(g) = lock {
                g.touch();
                let mut did_start = false;

                // If a daemon process exists but isn't ready yet, don't try to start a second
                // one (daemon::start_daemon would bail). Just wait for readiness.
                if !daemon::is_daemon_running() {
                    if daemon::start_daemon(&[]).is_ok() {
                        did_start = true;
                    } else {
                        return None;
                    }
                }

                // Wait for readiness (socket + /health).
                for _ in 0..240 {
                    if socket_path.exists() && rt.block_on(async { daemon_health_check().await }) {
                        ready = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }

                if ready && did_start {
                    eprintln!("\x1b[2m▸ daemon auto-started\x1b[0m");
                }
            } else {
                // Another process likely holds the start lock; wait briefly for readiness.
                for _ in 0..240 {
                    if socket_path.exists() && rt.block_on(async { daemon_health_check().await }) {
                        ready = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
        #[cfg(not(unix))]
        {
            return None;
        }
    }

    if !ready {
        return None;
    }

    if let Some(out) = rt.block_on(async { daemon_tool_call(name, arguments.as_ref()).await.ok() })
    {
        return Some(out);
    }

    // If the daemon is starting up, the first request can still lose a race even after /health
    // briefly succeeds. Retry once after a short wait.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(50));
        if let Some(out) =
            rt.block_on(async { daemon_tool_call(name, arguments.as_ref()).await.ok() })
        {
            return Some(out);
        }
    }

    None
}

fn unwrap_mcp_tool_text(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let result = v.get("result")?;

    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        let mut texts: Vec<String> = Vec::new();
        for item in content {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    texts.push(text.to_string());
                }
            }
        }
        if !texts.is_empty() {
            return Some(texts.join("\n"));
        }
    }

    if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
        return Some(text.to_string());
    }

    result.as_str().map(std::string::ToString::to_string)
}

/// Like `try_daemon_tool_call_blocking`, but unwraps MCP JSON responses to text for CLI output.
pub fn try_daemon_tool_call_blocking_text(
    name: &str,
    arguments: Option<serde_json::Value>,
) -> Option<String> {
    let body = try_daemon_tool_call_blocking(name, arguments)?;
    let trimmed = body.trim_start();
    if !trimmed.starts_with('{') {
        return Some(body);
    }
    Some(unwrap_mcp_tool_text(&body).unwrap_or(body))
}
