pub fn resolve_portable_binary() -> String {
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(status) = std::process::Command::new(which_cmd)
        .arg("lean-ctx")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
    {
        if status.success() {
            return "lean-ctx".to_string();
        }
    }
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "lean-ctx".to_string())
}
