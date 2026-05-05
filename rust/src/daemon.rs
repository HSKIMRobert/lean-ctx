use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("lean-ctx")
}

pub fn daemon_pid_path() -> PathBuf {
    data_dir().join("daemon.pid")
}

pub fn daemon_socket_path() -> PathBuf {
    data_dir().join("daemon.sock")
}

pub fn is_daemon_running() -> bool {
    let pid_path = daemon_pid_path();
    let Ok(contents) = fs::read_to_string(&pid_path) else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return false;
    };
    process_alive(pid)
}

pub fn read_daemon_pid() -> Option<u32> {
    let contents = fs::read_to_string(daemon_pid_path()).ok()?;
    contents.trim().parse::<u32>().ok()
}

pub fn start_daemon(args: &[String]) -> Result<()> {
    if is_daemon_running() {
        let pid = read_daemon_pid().unwrap_or(0);
        anyhow::bail!("Daemon already running (PID {pid}). Use --stop to stop it first.");
    }

    cleanup_stale_socket();

    let exe = std::env::current_exe().context("cannot determine own executable path")?;

    let mut cmd_args = vec!["serve".to_string()];
    for arg in args {
        if arg == "--daemon" || arg == "-d" {
            continue;
        }
        cmd_args.push(arg.clone());
    }
    cmd_args.push("--_foreground-daemon".to_string());

    let child = Command::new(&exe)
        .args(&cmd_args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn daemon: {}", exe.display()))?;

    let pid = child.id();
    write_pid_file(pid)?;

    std::thread::sleep(std::time::Duration::from_millis(200));

    if !process_alive(pid) {
        let _ = fs::remove_file(daemon_pid_path());
        anyhow::bail!("Daemon process exited immediately. Check logs for errors.");
    }

    eprintln!(
        "lean-ctx daemon started (PID {pid})\n  Socket: {}\n  PID file: {}",
        daemon_socket_path().display(),
        daemon_pid_path().display()
    );

    Ok(())
}

pub fn stop_daemon() -> Result<()> {
    let pid_path = daemon_pid_path();

    let Some(pid) = read_daemon_pid() else {
        eprintln!("No daemon PID file found. Nothing to stop.");
        return Ok(());
    };

    if !process_alive(pid) {
        eprintln!("Daemon (PID {pid}) is not running. Cleaning up stale files.");
        cleanup_stale_socket();
        let _ = fs::remove_file(&pid_path);
        return Ok(());
    }

    send_sigterm(pid)?;

    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !process_alive(pid) {
            break;
        }
    }

    if process_alive(pid) {
        eprintln!("Daemon (PID {pid}) did not stop gracefully, sending SIGKILL.");
        send_sigkill(pid)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let _ = fs::remove_file(&pid_path);
    cleanup_stale_socket();
    eprintln!("lean-ctx daemon stopped (PID {pid}).");
    Ok(())
}

pub fn daemon_status() -> String {
    if let Some(pid) = read_daemon_pid() {
        if process_alive(pid) {
            let sock = daemon_socket_path();
            let sock_exists = sock.exists();
            return format!(
                "Daemon running (PID {pid})\n  Socket: {} ({})\n  PID file: {}",
                sock.display(),
                if sock_exists { "ready" } else { "missing" },
                daemon_pid_path().display()
            );
        }
        return format!("Daemon not running (stale PID file for PID {pid})");
    }
    "Daemon not running".to_string()
}

fn write_pid_file(pid: u32) -> Result<()> {
    let pid_path = daemon_pid_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create dir: {}", parent.display()))?;
    }
    let mut f = fs::File::create(&pid_path)
        .with_context(|| format!("cannot write PID file: {}", pid_path.display()))?;
    write!(f, "{pid}")?;
    Ok(())
}

fn cleanup_stale_socket() {
    let sock = daemon_socket_path();
    if sock.exists() {
        let _ = fs::remove_file(&sock);
    }
}

fn process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

fn send_sigterm(pid: u32) -> Result<()> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if ret != 0 {
        anyhow::bail!(
            "Failed to send SIGTERM to PID {pid}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn send_sigkill(pid: u32) -> Result<()> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    if ret != 0 {
        anyhow::bail!(
            "Failed to send SIGKILL to PID {pid}: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

/// Write the current process's PID and setup signal handler for cleanup.
/// Called from the foreground-daemon process after fork.
pub fn init_foreground_daemon() -> Result<()> {
    let pid = std::process::id();
    write_pid_file(pid)?;
    Ok(())
}

/// Cleanup PID file and socket on shutdown.
pub fn cleanup_daemon_files() {
    let _ = fs::remove_file(daemon_pid_path());
    cleanup_stale_socket();
}
