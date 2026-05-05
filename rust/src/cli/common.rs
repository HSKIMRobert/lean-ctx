pub(crate) fn print_savings(original: usize, sent: usize) {
    let saved = original.saturating_sub(sent);
    if original > 0 && saved > 0 {
        let pct = (saved as f64 / original as f64 * 100.0).round() as usize;
        println!("[{saved} tok saved ({pct}%)]");
    }
}

pub fn load_shell_history_pub() -> Vec<String> {
    load_shell_history()
}

pub(crate) fn load_shell_history() -> Vec<String> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let history_file = if shell.contains("zsh") {
        home.join(".zsh_history")
    } else if shell.contains("fish") {
        home.join(".local/share/fish/fish_history")
    } else if cfg!(windows) && shell.is_empty() {
        home.join("AppData")
            .join("Roaming")
            .join("Microsoft")
            .join("Windows")
            .join("PowerShell")
            .join("PSReadLine")
            .join("ConsoleHost_history.txt")
    } else {
        home.join(".bash_history")
    };

    match std::fs::read_to_string(&history_file) {
        Ok(content) => content
            .lines()
            .filter_map(|l| {
                let trimmed = l.trim();
                if trimmed.starts_with(':') {
                    trimmed
                        .split(';')
                        .nth(1)
                        .map(std::string::ToString::to_string)
                } else {
                    Some(trimmed.to_string())
                }
            })
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub(crate) fn daemon_fallback_hint() {
    use std::sync::Once;
    static HINT: Once = Once::new();
    HINT.call_once(|| {
        eprintln!("\x1b[2;33mhint: daemon not running — stats tracked locally (lean-ctx serve -d for full tracking)\x1b[0m");
    });
}

pub(crate) fn format_tokens_cli(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

pub(crate) fn cli_track_read(path: &str, mode: &str, original_tokens: usize, output_tokens: usize) {
    crate::core::stats::record(&format!("cli_{mode}"), original_tokens, output_tokens);
    crate::core::heatmap::record_file_access(
        path,
        original_tokens,
        original_tokens.saturating_sub(output_tokens),
    );
}

pub(crate) fn cli_track_search(original_tokens: usize, output_tokens: usize) {
    crate::core::stats::record("cli_grep", original_tokens, output_tokens);
}

pub(crate) fn cli_track_tree(original_tokens: usize, output_tokens: usize) {
    crate::core::stats::record("cli_ls", original_tokens, output_tokens);
}

pub(crate) fn detect_project_root(args: &[String]) -> String {
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if let Some(v) = a.strip_prefix("--root=") {
            if !v.trim().is_empty() {
                return v.to_string();
            }
        }
        if let Some(v) = a.strip_prefix("--project-root=") {
            if !v.trim().is_empty() {
                return v.to_string();
            }
        }
        if a == "--root" || a == "--project-root" {
            if let Some(v) = it.peek() {
                if !v.starts_with("--") && !v.trim().is_empty() {
                    return (*v).clone();
                }
            }
        }
    }
    std::env::current_dir()
        .ok()
        .map_or_else(|| ".".to_string(), |p| p.to_string_lossy().to_string())
}
