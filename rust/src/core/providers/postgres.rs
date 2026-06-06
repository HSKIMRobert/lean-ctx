//! PostgreSQL provider — database schema introspection via `psql`.
//!
//! Extracts table/column definitions from `information_schema` to make
//! database structure available as context. Uses `psql` CLI to avoid
//! adding a native PG driver dependency.
//!
//! Configuration via environment variables:
//!   - `DATABASE_URL`: Full connection string (e.g., "postgres://user:pass@host/db")
//!   - Or individual: `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGPASSWORD`

use crate::core::providers::{ContextProvider, ProviderItem, ProviderParams, ProviderResult};

pub struct PostgresProvider {
    available: bool,
}

impl Default for PostgresProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresProvider {
    pub fn new() -> Self {
        let available =
            std::env::var("DATABASE_URL").is_ok() || std::env::var("PGDATABASE").is_ok();
        Self { available }
    }
}

impl ContextProvider for PostgresProvider {
    fn id(&self) -> &'static str {
        "postgres"
    }

    fn display_name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn supported_actions(&self) -> &[&str] {
        &["schemas", "tables"]
    }

    fn execute(&self, action: &str, params: &ProviderParams) -> Result<ProviderResult, String> {
        if !self.available {
            return Err("PostgreSQL not configured (need DATABASE_URL or PGDATABASE)".into());
        }
        match action {
            "schemas" | "tables" => list_tables(params),
            _ => Err(format!("Unsupported action: {action}")),
        }
    }

    fn cache_ttl_secs(&self) -> u64 {
        300
    }

    fn requires_auth(&self) -> bool {
        true
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

fn list_tables(params: &ProviderParams) -> Result<ProviderResult, String> {
    let schema = params.state.as_deref().unwrap_or("public");
    let limit = params.limit.unwrap_or(50);

    let query = format!(
        "SELECT table_name, column_name, data_type, is_nullable \
         FROM information_schema.columns \
         WHERE table_schema = '{schema}' \
         ORDER BY table_name, ordinal_position \
         LIMIT {limit_cols};",
        limit_cols = limit * 20, // ~20 columns per table avg
    );

    let mut cmd = std::process::Command::new("psql");

    if let Ok(url) = std::env::var("DATABASE_URL") {
        cmd.arg(&url);
    }

    let output = cmd
        .args(["-t", "-A", "-F", "|", "-c", &query])
        .output()
        .map_err(|e| format!("Failed to run psql: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("psql error: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tables: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let table = parts[0].trim();
            let col = parts[1].trim();
            let dtype = parts[2].trim();
            let nullable = parts.get(3).map_or("", |s| s.trim());

            let null_marker = if nullable == "YES" { "?" } else { "" };
            tables
                .entry(table.to_string())
                .or_default()
                .push(format!("  {col}: {dtype}{null_marker}"));
        }
    }

    let items: Vec<ProviderItem> = tables
        .iter()
        .take(limit)
        .map(|(table, columns)| {
            let body = format!("{schema}.{table}\n{}", columns.join("\n"));
            ProviderItem {
                id: table.clone(),
                title: format!("{schema}.{table}"),
                state: Some("active".into()),
                author: None,
                created_at: None,
                updated_at: None,
                url: None,
                labels: vec![schema.to_string()],
                body: Some(body),
                ..Default::default()
            }
        })
        .collect();

    Ok(ProviderResult {
        provider: "postgres".into(),
        resource_type: "schemas".into(),
        items,
        total_count: Some(tables.len()),
        truncated: tables.len() > limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_provider_unavailable_without_env() {
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("PGDATABASE");

        let provider = PostgresProvider::new();
        assert!(!provider.is_available());
        assert_eq!(provider.id(), "postgres");
        assert!(provider.requires_auth());
    }

    #[test]
    fn postgres_provider_supported_actions() {
        let provider = PostgresProvider::new();
        assert!(provider.supported_actions().contains(&"schemas"));
        assert!(provider.supported_actions().contains(&"tables"));
    }
}
