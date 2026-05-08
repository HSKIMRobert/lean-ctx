use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const DEFAULT_PORT: u16 = 3333;
const DEFAULT_HOST: &str = "127.0.0.1";
const DASHBOARD_HTML: &str = include_str!("dashboard.html");

const COCKPIT_INDEX_HTML: &str = include_str!("static/index.html");
const COCKPIT_STYLE_CSS: &str = include_str!("static/style.css");
const COCKPIT_LIB_API_JS: &str = include_str!("static/lib/api.js");
const COCKPIT_LIB_FORMAT_JS: &str = include_str!("static/lib/format.js");
const COCKPIT_LIB_ROUTER_JS: &str = include_str!("static/lib/router.js");
const COCKPIT_LIB_CHARTS_JS: &str = include_str!("static/lib/charts.js");
const COCKPIT_LIB_SHARED_JS: &str = include_str!("static/lib/shared.js");
const COCKPIT_COMPONENT_NAV_JS: &str = include_str!("static/components/cockpit-nav.js");
const COCKPIT_COMPONENT_CONTEXT_JS: &str = include_str!("static/components/cockpit-context.js");
const COCKPIT_COMPONENT_OVERVIEW_JS: &str = include_str!("static/components/cockpit-overview.js");
const COCKPIT_COMPONENT_LIVE_JS: &str = include_str!("static/components/cockpit-live.js");
const COCKPIT_COMPONENT_KNOWLEDGE_JS: &str = include_str!("static/components/cockpit-knowledge.js");
const COCKPIT_COMPONENT_AGENTS_JS: &str = include_str!("static/components/cockpit-agents.js");
const COCKPIT_COMPONENT_MEMORY_JS: &str = include_str!("static/components/cockpit-memory.js");
const COCKPIT_COMPONENT_SEARCH_JS: &str = include_str!("static/components/cockpit-search.js");
const COCKPIT_COMPONENT_COMPRESSION_JS: &str =
    include_str!("static/components/cockpit-compression.js");
const COCKPIT_COMPONENT_GRAPH_JS: &str = include_str!("static/components/cockpit-graph.js");
const COCKPIT_COMPONENT_HEALTH_JS: &str = include_str!("static/components/cockpit-health.js");
const COCKPIT_COMPONENT_REMAINING_JS: &str = include_str!("static/components/cockpit-remaining.js");

pub mod routes;

pub async fn start(port: Option<u16>, host: Option<String>) {
    let port = port.unwrap_or_else(|| {
        std::env::var("LEAN_CTX_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_PORT)
    });

    let host = host.unwrap_or_else(|| {
        std::env::var("LEAN_CTX_HOST")
            .ok()
            .unwrap_or_else(|| DEFAULT_HOST.to_string())
    });

    let addr = format!("{host}:{port}");
    let is_local = host == "127.0.0.1" || host == "localhost" || host == "::1";

    // Avoid accidental multiple dashboard instances (common source of "it hangs").
    // Only safe to auto-detect for local dashboards without auth.
    if is_local && dashboard_responding(&host, port) {
        println!("\n  lean-ctx dashboard already running → http://{host}:{port}");
        println!("  Tip: use Ctrl+C in the existing terminal to stop it.\n");
        let saved = load_saved_token();
        if let Some(ref t) = saved {
            open_browser(&format!("http://localhost:{port}/?token={t}"));
        } else {
            open_browser(&format!("http://localhost:{port}"));
        }
        return;
    }

    // Always enable auth (even on loopback) to prevent cross-origin reads of /api/*
    // from a malicious website (CORS is not a reliable boundary for localhost services).
    let t = generate_token();
    save_token(&t);
    let token = Some(Arc::new(t));

    if let Some(t) = token.as_ref() {
        if is_local {
            println!("  Auth: enabled (local)");
            println!("  Browser URL:  http://localhost:{port}/?token={t}");
        } else {
            eprintln!(
                "  \x1b[33m⚠\x1b[0m Binding to {host} — authentication enabled.\n  \
                 Bearer token: \x1b[1;32m{t}\x1b[0m\n  \
                 Browser URL:  http://<your-ip>:{port}/?token={t}"
            );
        }
    }

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {addr}: {e}");
            std::process::exit(1);
        }
    };

    let stats_path = crate::core::data_dir::lean_ctx_data_dir().map_or_else(
        |_| "~/.lean-ctx/stats.json".to_string(),
        |d| d.join("stats.json").display().to_string(),
    );

    if host == "0.0.0.0" {
        println!("\n  lean-ctx dashboard → http://0.0.0.0:{port} (all interfaces)");
        println!("  Local access:  http://localhost:{port}");
    } else {
        println!("\n  lean-ctx dashboard → http://{host}:{port}");
    }
    println!("  Stats file: {stats_path}");
    println!("  Press Ctrl+C to stop\n");

    if is_local {
        if let Some(t) = token.as_ref() {
            open_browser(&format!("http://localhost:{port}/?token={t}"));
        } else {
            open_browser(&format!("http://localhost:{port}"));
        }
    }
    if crate::shell::is_container() && is_local {
        println!("  Tip (Docker): bind 0.0.0.0 + publish port:");
        println!("    lean-ctx dashboard --host=0.0.0.0 --port={port}");
        println!("    docker run ... -p {port}:{port} ...");
        println!();
    }

    loop {
        if let Ok((stream, _)) = listener.accept().await {
            let token_ref = token.clone();
            tokio::spawn(handle_request(stream, token_ref));
        }
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    let _ = getrandom::fill(&mut bytes);
    format!("lctx_{}", hex_lower(&bytes))
}

fn save_token(token: &str) {
    if let Ok(dir) = crate::core::data_dir::lean_ctx_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("dashboard.token");
        let _ = std::fs::write(&path, token);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

fn load_saved_token() -> Option<String> {
    let dir = crate::core::data_dir::lean_ctx_data_dir().ok()?;
    let path = dir.join("dashboard.token");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(url)
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();
    }
}

fn dashboard_responding(host: &str, port: u16) -> bool {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("{host}:{port}");
    let Ok(mut s) = TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], port))),
        Duration::from_millis(150),
    ) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(150)));
    let _ = s.set_write_timeout(Some(Duration::from_millis(150)));

    let req = "GET /api/version HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if s.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 256];
    let Ok(n) = s.read(&mut buf) else {
        return false;
    };
    let head = String::from_utf8_lossy(&buf[..n]);
    head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200")
}

const MAX_HTTP_MESSAGE: usize = 2 * 1024 * 1024;

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length_header(header_section: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(header_section);
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case("content-length") {
            return v.trim().parse::<usize>().ok();
        }
    }
    Some(0)
}

async fn read_http_message(stream: &mut tokio::net::TcpStream) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        if let Some(end) = find_headers_end(&buf) {
            let cl = parse_content_length_header(&buf[..end])?;
            let total = end + 4 + cl;
            if total > MAX_HTTP_MESSAGE {
                return None;
            }
            if buf.len() >= total {
                buf.truncate(total);
                return Some(buf);
            }
        } else if buf.len() > 65_536 {
            return None;
        }

        let n = stream.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX_HTTP_MESSAGE {
            return None;
        }
    }
}

async fn handle_request(mut stream: tokio::net::TcpStream, token: Option<Arc<String>>) {
    let Some(buf) = read_http_message(&mut stream).await else {
        return;
    };
    let Some(header_end) = find_headers_end(&buf) else {
        return;
    };
    let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let body_start = header_end + 4;
    let Some(content_len) = parse_content_length_header(&buf[..header_end]) else {
        return;
    };
    if buf.len() < body_start + content_len {
        return;
    }
    let body_str = std::str::from_utf8(&buf[body_start..body_start + content_len])
        .unwrap_or("")
        .to_string();

    let first = header_text.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let raw_path = parts.next().unwrap_or("/").to_string();

    let (path, query_token) = if let Some(idx) = raw_path.find('?') {
        let p = &raw_path[..idx];
        let qs = &raw_path[idx + 1..];
        let tok = qs
            .split('&')
            .find_map(|pair| pair.strip_prefix("token="))
            .map(std::string::ToString::to_string);
        (p.to_string(), tok)
    } else {
        (raw_path.clone(), None)
    };

    let query_str = raw_path
        .find('?')
        .map_or(String::new(), |i| raw_path[i + 1..].to_string());

    let is_api = path.starts_with("/api/");
    let requires_auth = is_api || path == "/metrics";

    if let Some(ref expected) = token {
        let has_header_auth = check_auth(&header_text, expected);

        if requires_auth && !has_header_auth {
            let body = r#"{"error":"unauthorized"}"#;
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 WWW-Authenticate: Bearer\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            return;
        }
    }

    let path = path.as_str();
    let query_str = query_str.as_str();
    let method = method.as_str();

    let compute = std::panic::catch_unwind(|| {
        routes::route_response(
            path,
            query_str,
            query_token.as_ref(),
            token.as_ref(),
            method,
            &body_str,
        )
    });
    let (status, content_type, body) = match compute {
        Ok(v) => v,
        Err(_) => (
            "500 Internal Server Error",
            "application/json",
            r#"{"error":"dashboard route panicked"}"#.to_string(),
        ),
    };

    let cache_header = if content_type.starts_with("application/json") {
        "Cache-Control: no-cache, no-store, must-revalidate\r\nPragma: no-cache\r\n"
    } else {
        ""
    };

    let security_headers = "\
        X-Content-Type-Options: nosniff\r\n\
        X-Frame-Options: DENY\r\n\
        Referrer-Policy: no-referrer\r\n\
        Content-Security-Policy: default-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net https://fonts.googleapis.com https://fonts.gstatic.com; img-src 'self' data:; font-src 'self' https://fonts.gstatic.com\r\n";

    let response = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         {cache_header}\
         {security_headers}\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );

    let _ = stream.write_all(response.as_bytes()).await;
}

fn check_auth(request: &str, expected_token: &str) -> bool {
    for line in request.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("authorization:") {
            let value = line["authorization:".len()..].trim();
            if let Some(token) = value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
            {
                return constant_time_eq(token.trim().as_bytes(), expected_token.as_bytes());
            }
        }
    }
    false
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::routes::helpers::normalize_dashboard_demo_path;
    use super::*;
    use tempfile::tempdir;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn check_auth_with_valid_bearer() {
        let req = "GET /api/stats HTTP/1.1\r\nAuthorization: Bearer lctx_abc123\r\n\r\n";
        assert!(check_auth(req, "lctx_abc123"));
    }

    #[test]
    fn check_auth_with_invalid_bearer() {
        let req = "GET /api/stats HTTP/1.1\r\nAuthorization: Bearer wrong_token\r\n\r\n";
        assert!(!check_auth(req, "lctx_abc123"));
    }

    #[test]
    fn check_auth_missing_header() {
        let req = "GET /api/stats HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert!(!check_auth(req, "lctx_abc123"));
    }

    #[test]
    fn check_auth_lowercase_bearer() {
        let req = "GET /api/stats HTTP/1.1\r\nauthorization: bearer lctx_abc123\r\n\r\n";
        assert!(check_auth(req, "lctx_abc123"));
    }

    #[test]
    fn query_token_parsing() {
        let raw_path = "/index.html?token=lctx_abc123&other=val";
        let idx = raw_path.find('?').unwrap();
        let qs = &raw_path[idx + 1..];
        let tok = qs.split('&').find_map(|pair| pair.strip_prefix("token="));
        assert_eq!(tok, Some("lctx_abc123"));
    }

    #[test]
    fn api_path_detection() {
        assert!("/api/stats".starts_with("/api/"));
        assert!("/api/version".starts_with("/api/"));
        assert!(!"/".starts_with("/api/"));
        assert!(!"/index.html".starts_with("/api/"));
        assert!(!"/favicon.ico".starts_with("/api/"));
    }

    #[test]
    fn normalize_dashboard_demo_path_strips_rooted_relative_windows_path() {
        let normalized = normalize_dashboard_demo_path(r"\backend\list_tables.js");
        assert_eq!(
            normalized,
            format!("backend{}list_tables.js", std::path::MAIN_SEPARATOR)
        );
    }

    #[test]
    fn normalize_dashboard_demo_path_preserves_absolute_windows_path() {
        let input = r"C:\repo\backend\list_tables.js";
        assert_eq!(normalize_dashboard_demo_path(input), input);
    }

    #[test]
    fn normalize_dashboard_demo_path_preserves_unc_path() {
        let input = r"\\server\share\backend\list_tables.js";
        assert_eq!(normalize_dashboard_demo_path(input), input);
    }

    #[test]
    fn normalize_dashboard_demo_path_strips_dot_slash_prefix() {
        assert_eq!(
            normalize_dashboard_demo_path("./src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            normalize_dashboard_demo_path(r".\src\main.rs"),
            format!("src{}main.rs", std::path::MAIN_SEPARATOR)
        );
    }

    #[test]
    fn api_profile_returns_json() {
        let (_status, _ct, body) =
            routes::route_response("/api/profile", "", None, None, "GET", "");
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(v.get("active_name").is_some(), "missing active_name");
        assert!(
            v.pointer("/profile/profile/name")
                .and_then(|n| n.as_str())
                .is_some(),
            "missing profile.profile.name"
        );
        assert!(v.get("available").and_then(|a| a.as_array()).is_some());
    }

    #[test]
    fn api_episodes_returns_json() {
        let (_status, _ct, body) =
            routes::route_response("/api/episodes", "", None, None, "GET", "");
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(v.get("project_hash").is_some());
        assert!(v.get("stats").is_some());
        assert!(v.get("recent").and_then(|a| a.as_array()).is_some());
    }

    #[test]
    fn api_procedures_returns_json() {
        let (_status, _ct, body) =
            routes::route_response("/api/procedures", "", None, None, "GET", "");
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(v.get("project_hash").is_some());
        assert!(v.get("procedures").and_then(|a| a.as_array()).is_some());
        assert!(v.get("suggestions").and_then(|a| a.as_array()).is_some());
    }

    #[test]
    fn api_compression_demo_heals_moved_file_paths() {
        let _g = ENV_LOCK.lock().expect("env lock");
        let td = tempdir().expect("tempdir");
        let root = td.path();
        std::fs::create_dir_all(root.join("src").join("moved")).expect("mkdir");
        std::fs::write(
            root.join("src").join("moved").join("foo.rs"),
            "pub fn foo() { println!(\"hi\"); }\n",
        )
        .expect("write foo.rs");

        let root_s = root.to_string_lossy().to_string();
        std::env::set_var("LEAN_CTX_DASHBOARD_PROJECT", &root_s);

        let (_status, _ct, body) = routes::route_response(
            "/api/compression-demo",
            "path=src/foo.rs",
            None,
            None,
            "GET",
            "",
        );
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(v.get("error").is_none(), "unexpected error: {body}");
        assert_eq!(
            v.get("resolved_from").and_then(|x| x.as_str()),
            Some("src/moved/foo.rs")
        );

        std::env::remove_var("LEAN_CTX_DASHBOARD_PROJECT");
        if let Some(dir) = crate::core::graph_index::ProjectIndex::index_dir(&root_s) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
