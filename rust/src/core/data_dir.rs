use std::path::PathBuf;

pub fn lean_ctx_data_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("LEAN_CTX_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    Ok(dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?
        .join(".lean-ctx"))
}
