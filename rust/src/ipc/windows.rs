use anyhow::{Context, Result};

pub(super) fn default_pipe_name() -> String {
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "default".to_string());
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("AppData/Local"))
        .join("lean-ctx");
    let seed = format!("{username}:{}", data_dir.display());
    let hash = blake3::hash(seed.as_bytes());
    let short = &hash.to_hex()[..16];
    format!(r"\\.\pipe\lean-ctx-{short}")
}

pub(super) fn pipe_exists(name: &str) -> bool {
    use std::fs;
    fs::metadata(name).is_ok()
}

pub(super) async fn connect(
    pipe_name: &str,
) -> Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    use tokio::net::windows::named_pipe::ClientOptions;

    ClientOptions::new()
        .open(pipe_name)
        .with_context(|| format!("connect to daemon pipe {pipe_name}"))
}
