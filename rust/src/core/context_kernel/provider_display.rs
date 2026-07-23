//! Dashboard and CLI formatting for per-provider token statistics.

use serde::Serialize;
use serde_json::Value;

use super::envelope_bridge::ProviderStat;
use super::provider_parity;

const HEADER: &str = "Provider     │ Reqs │    Input │   Output │   Cache │ Avg In";
const RULE: &str = "─────────────┼──────┼──────────┼──────────┼─────────┼───────";

#[derive(Serialize)]
struct ProviderJsonRow {
    provider_name: &'static str,
    request_count: usize,
    total_input: usize,
    total_output: usize,
    total_cache_read: usize,
    avg_input: usize,
}

fn separated(value: usize) -> String {
    let mut formatted = value.to_string();
    let mut index = formatted.len();
    while index > 3 {
        index -= 3;
        formatted.insert(index, ',');
    }
    formatted
}

fn average_input(stat: &ProviderStat) -> usize {
    stat.total_input
        .checked_div(stat.request_count)
        .unwrap_or(0)
}

fn row(name: &str, values: [usize; 5]) -> String {
    let values = values.map(separated);
    format!(
        "{name:<12} │ {:>4} │ {:>8} │ {:>8} │ {:>7} │ {:>6}",
        values[0], values[1], values[2], values[3], values[4]
    )
}

/// Formats provider statistics as a multi-line dashboard table.
pub fn format_provider_table(stats: &[ProviderStat]) -> String {
    if stats.is_empty() {
        return "No provider data available.".to_owned();
    }
    let mut lines = vec![HEADER.to_owned(), RULE.to_owned()];
    let mut totals = (0usize, 0usize, 0usize, 0usize);
    for stat in stats {
        totals.0 = totals.0.saturating_add(stat.request_count);
        totals.1 = totals.1.saturating_add(stat.total_input);
        totals.2 = totals.2.saturating_add(stat.total_output);
        totals.3 = totals.3.saturating_add(stat.total_cache_read);
        let input = stat.total_input;
        lines.push(row(
            provider_parity::provider_display_name(stat.provider),
            [
                stat.request_count,
                input,
                stat.total_output,
                stat.total_cache_read,
                average_input(stat),
            ],
        ));
    }
    lines.push(RULE.to_owned());
    let (requests, input, output, cache) = totals;
    lines.push(row(
        "Total",
        [
            requests,
            input,
            output,
            cache,
            input.checked_div(requests).unwrap_or(0),
        ],
    ));
    lines.join("\n")
}

/// Summarizes provider request counts on one line.
pub fn provider_summary_oneliner(stats: &[ProviderStat]) -> String {
    if stats.is_empty() {
        return "No provider data".to_owned();
    }
    let providers = stats
        .iter()
        .map(|stat| {
            format!(
                "{}({})",
                provider_parity::provider_display_name(stat.provider),
                stat.request_count
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{} providers: {providers}", stats.len())
}

/// Serializes provider statistics into dashboard-ready JSON.
pub fn provider_json(stats: &[ProviderStat]) -> Value {
    let rows = stats.iter().map(|stat| ProviderJsonRow {
        provider_name: provider_parity::provider_display_name(stat.provider),
        request_count: stat.request_count,
        total_input: stat.total_input,
        total_output: stat.total_output,
        total_cache_read: stat.total_cache_read,
        avg_input: average_input(stat),
    });
    serde_json::to_value(rows.collect::<Vec<_>>()).unwrap_or_else(|_| Value::Array(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::{format_provider_table, provider_json, provider_summary_oneliner};
    use crate::core::context_kernel::envelope_bridge::ProviderStat;
    use crate::core::context_kernel::token_envelope::ProviderKind;

    fn stats() -> [ProviderStat; 2] {
        let open_ai = ProviderStat {
            provider: ProviderKind::OpenAi,
            request_count: 42,
            total_input: 125_400,
            ..ProviderStat::default()
        };
        let anthropic = ProviderStat {
            provider: ProviderKind::Anthropic,
            request_count: 18,
            total_input: 54_000,
            ..ProviderStat::default()
        };
        [open_ai, anthropic]
    }

    #[test]
    fn table_with_data() {
        let table = format_provider_table(&stats());
        assert!(table.contains("│") && table.contains("OpenAI") && table.contains("Anthropic"));
    }

    #[test]
    fn table_empty() {
        assert_eq!(format_provider_table(&[]), "No provider data available.");
    }

    #[test]
    fn oneliner_format() {
        let line = provider_summary_oneliner(&stats());
        assert!(line.contains("OpenAI(42)") && line.contains("Anthropic(18)"));
    }

    #[test]
    fn json_array_valid() {
        let json = provider_json(&stats());
        assert!(json.is_array());
        assert_eq!(json[0]["provider_name"], "OpenAI");
        assert_eq!(json[0]["request_count"], 42);
        assert_eq!(json[0]["avg_input"], 2_985);
    }

    #[test]
    fn oneliner_empty() {
        assert_eq!(provider_summary_oneliner(&[]), "No provider data");
    }
}
