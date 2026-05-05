pub fn handle_stats(format: Option<&str>) -> Result<String, String> {
    let snap = crate::core::verification_observability::snapshot_v1();
    match format.unwrap_or("summary") {
        "json" => Ok(serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())?),
        "both" => Ok(format!(
            "{}\n\n{}",
            crate::core::verification_observability::format_compact(&snap),
            serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())?
        )),
        _ => Ok(crate::core::verification_observability::format_compact(
            &snap,
        )),
    }
}
