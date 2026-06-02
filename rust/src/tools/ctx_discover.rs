use std::collections::HashMap;

use crate::core::tokens::count_tokens;

const COMPRESSIBLE_COMMANDS: &[(&str, &str, &str)] = &[
    ("git", "git status/diff/log/add/commit/push", "80-95%"),
    ("cargo", "cargo build/test/clippy", "80-95%"),
    ("npm", "npm install/run/test", "60-85%"),
    ("pnpm", "pnpm install/run/test", "60-85%"),
    ("yarn", "yarn install/run/test", "60-85%"),
    ("docker", "docker ps/images/logs/build", "60-80%"),
    ("kubectl", "kubectl get/describe/logs", "60-80%"),
    ("pip", "pip install/list/freeze", "60-85%"),
    ("go", "go test/build/vet", "75-90%"),
    ("ruff", "ruff check/format", "80-90%"),
    ("eslint", "eslint/biome lint", "80-90%"),
    ("prettier", "prettier --check", "70-80%"),
    ("tsc", "TypeScript compiler", "80-90%"),
    ("curl", "HTTP requests", "60-80%"),
    ("grep", "grep/rg search", "50-80%"),
    ("find", "find files", "50-70%"),
    ("ls", "directory listing", "50-70%"),
    ("pytest", "Python tests", "85-95%"),
    ("rspec", "Ruby tests", "60-80%"),
    ("aws", "AWS CLI", "60-80%"),
    ("helm", "Kubernetes Helm", "60-80%"),
    ("terraform", "Terraform", "60-80%"),
    ("ansible", "Ansible", "60-80%"),
    ("prisma", "Prisma ORM", "70-85%"),
    ("cmake", "CMake build", "60-80%"),
    ("bazel", "Bazel build", "60-80%"),
    ("zig", "Zig build/test", "60-80%"),
    ("swift", "Swift build/test", "60-80%"),
    ("deno", "Deno runtime", "60-80%"),
    ("bun", "Bun runtime", "60-80%"),
    ("composer", "PHP Composer", "60-80%"),
    ("mix", "Elixir Mix", "60-80%"),
    ("php", "PHP CLI/artisan", "60-80%"),
];

pub struct DiscoverResult {
    pub total_commands: u32,
    pub already_optimized: u32,
    pub missed_commands: Vec<MissedCommand>,
    pub potential_tokens: usize,
    pub potential_usd: f64,
}

pub struct MissedCommand {
    pub prefix: String,
    pub description: String,
    pub savings_range: String,
    pub count: u32,
    pub estimated_tokens: usize,
}

pub fn analyze_history(history: &[String], limit: usize) -> DiscoverResult {
    let mut missed: HashMap<&str, u32> = HashMap::new();
    let mut already_optimized = 0u32;
    let mut total_commands = 0u32;

    for cmd in history {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_commands += 1;

        if trimmed.starts_with("lean-ctx ") {
            already_optimized += 1;
            continue;
        }

        for (prefix, _, _) in COMPRESSIBLE_COMMANDS {
            if trimmed.starts_with(prefix) || trimmed.starts_with(&format!("{prefix} ")) {
                *missed.entry(prefix).or_insert(0) += 1;
                break;
            }
        }
    }

    let mut sorted: Vec<_> = missed.into_iter().collect();
    sorted.sort_by_key(|x| std::cmp::Reverse(x.1));

    let total_missed: u32 = sorted.iter().map(|(_, c)| c).sum();
    let est_tokens_per_cmd = 500;
    let est_savings_pct = 0.75;
    let potential = (total_missed as f64 * est_tokens_per_cmd as f64 * est_savings_pct) as usize;
    let potential_usd =
        potential as f64 * crate::core::stats::DEFAULT_INPUT_PRICE_PER_M / 1_000_000.0;

    let real_stats = crate::core::stats::load();
    let (effective_potential, effective_usd) = if real_stats.total_commands > 0 {
        let real_savings_rate = if real_stats.total_input_tokens > 0 {
            1.0 - (real_stats.total_output_tokens as f64 / real_stats.total_input_tokens as f64)
        } else {
            est_savings_pct
        };
        let p = (total_missed as f64 * est_tokens_per_cmd as f64 * real_savings_rate) as usize;
        let u = p as f64 * crate::core::stats::DEFAULT_INPUT_PRICE_PER_M / 1_000_000.0;
        (p, u)
    } else {
        (potential, potential_usd)
    };

    let missed_commands = sorted
        .into_iter()
        .take(limit)
        .map(|(prefix, count)| {
            let (desc, savings) = COMPRESSIBLE_COMMANDS
                .iter()
                .find(|(p, _, _)| p == &prefix)
                .map(|(_, d, s)| (d.to_string(), s.to_string()))
                .unwrap_or_default();
            MissedCommand {
                prefix: prefix.to_string(),
                description: desc,
                savings_range: savings,
                count,
                estimated_tokens: (count as f64 * est_tokens_per_cmd as f64 * est_savings_pct)
                    as usize,
            }
        })
        .collect();

    DiscoverResult {
        total_commands,
        already_optimized,
        missed_commands,
        potential_tokens: effective_potential,
        potential_usd: effective_usd,
    }
}

pub fn discover_from_history(history: &[String], limit: usize) -> String {
    let result = analyze_history(history, limit);

    if result.missed_commands.is_empty() {
        return format!(
            "No missed savings found in last {} commands. \
            {} already optimized.",
            result.total_commands, result.already_optimized
        );
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "Analyzed {} commands ({} already optimized):",
        result.total_commands, result.already_optimized
    ));
    lines.push(String::new());

    let total_missed: u32 = result.missed_commands.iter().map(|m| m.count).sum();
    lines.push(format!(
        "{total_missed} commands could benefit from lean-ctx:"
    ));
    lines.push(String::new());

    for m in &result.missed_commands {
        lines.push(format!(
            "  {:>4}x  {:<12} {} ({})",
            m.count, m.prefix, m.description, m.savings_range
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "Estimated potential: ~{} tokens saved (~${:.2})",
        result.potential_tokens, result.potential_usd
    ));
    lines.push(String::new());
    lines.push("Fix: run 'lean-ctx init --global' to auto-compress all commands.".to_string());
    lines.push("Or:  run 'lean-ctx init --agent <tool>' for AI tool hooks.".to_string());

    let output = lines.join("\n");
    let tokens = count_tokens(&output);
    format!("{output}\n\n[{tokens} tok]")
}

pub fn format_cli_output(result: &DiscoverResult) -> String {
    if result.missed_commands.is_empty() {
        return format!(
            "All compressible commands are already using lean-ctx!\n\
             ({} commands analyzed, {} via lean-ctx)",
            result.total_commands, result.already_optimized
        );
    }

    let mut lines = Vec::new();
    let total_missed: u32 = result.missed_commands.iter().map(|m| m.count).sum();

    lines.push(format!(
        "Found {total_missed} compressible commands not using lean-ctx:\n"
    ));
    lines.push(format!(
        "  {:<14} {:>5}  {:>10}  {:<30} {}",
        "COMMAND", "COUNT", "SAVINGS", "DESCRIPTION", "EST. TOKENS"
    ));
    lines.push(format!("  {}", "-".repeat(80)));

    for m in &result.missed_commands {
        lines.push(format!(
            "  {:<14} {:>5}x {:>10}  {:<30} ~{}",
            m.prefix, m.count, m.savings_range, m.description, m.estimated_tokens
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "Estimated missed savings: ~{} tokens (~${:.2}/month at current rate)",
        result.potential_tokens,
        result.potential_usd * 30.0
    ));
    lines.push(format!(
        "Already using lean-ctx: {} commands",
        result.already_optimized
    ));
    lines.push(String::new());
    lines.push("Run 'lean-ctx init --global' to enable compression for all commands.".to_string());

    lines.join("\n")
}

/// Renders a shareable "before lean-ctx" SVG card from a discover analysis — the
/// "ghost tokens you're leaving on the table" framing that drives the first-run share
/// loop. Same 1200x630 social-card dimensions and visual language as the Wrapped card,
/// but in an amber/red "leak" palette. Pure string building; all data-derived text is
/// XML-escaped. Aggregate estimates only — never command contents or arguments.
pub fn render_before_card(result: &DiscoverResult) -> String {
    let saved = crate::core::wrapped::format_tokens(result.potential_tokens as u64);
    let monthly_usd = result.potential_usd * 30.0;
    let total_missed: u32 = result.missed_commands.iter().map(|m| m.count).sum();
    let top = before_card_top_commands(result);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630" font-family="Inter, system-ui, -apple-system, Segoe UI, Roboto, sans-serif">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#0b1020"/>
      <stop offset="1" stop-color="#131a2e"/>
    </linearGradient>
    <linearGradient id="accent" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#f59e0b"/>
      <stop offset="1" stop-color="#ef4444"/>
    </linearGradient>
  </defs>
  <rect width="1200" height="630" fill="url(#bg)"/>
  <rect x="0" y="0" width="1200" height="8" fill="url(#accent)"/>
  <text x="70" y="92" fill="#e5e7eb" font-size="34" font-weight="700">lean-ctx <tspan fill="#f59e0b">Ghost Tokens</tspan></text>
  <text x="70" y="130" fill="#94a3b8" font-size="24">before lean-ctx — estimated from my shell history</text>
  <text x="70" y="300" fill="#f59e0b" font-size="120" font-weight="800" font-family="ui-monospace, SFMono-Regular, Menlo, monospace">{saved}</text>
  <text x="76" y="346" fill="#94a3b8" font-size="26">tokens/month left on the table</text>
  <text x="70" y="430" fill="#e5e7eb" font-size="60" font-weight="800" font-family="ui-monospace, SFMono-Regular, Menlo, monospace">${monthly_usd:.0}</text>
  <text x="74" y="462" fill="#94a3b8" font-size="22">potential monthly savings</text>
  <text x="70" y="512" fill="#cbd5e1" font-size="22">{total_missed} uncompressed commands · {already} already via lean-ctx</text>
{top}
  <text x="70" y="600" fill="#475569" font-size="17">Estimate from local shell history · run `lean-ctx setup` to stop the leak</text>
  <text x="1130" y="600" text-anchor="end" fill="#f59e0b" font-size="26" font-weight="700">leanctx.com</text>
</svg>"##,
        already = result.already_optimized,
    )
}

/// The top three missed commands as a single muted line. Empty when none.
fn before_card_top_commands(result: &DiscoverResult) -> String {
    if result.missed_commands.is_empty() {
        return String::new();
    }
    let joined = result
        .missed_commands
        .iter()
        .take(3)
        .map(|m| format!("{} {}x", m.prefix, m.count))
        .collect::<Vec<_>>()
        .join("    ·    ");
    format!(
        "  <text x=\"70\" y=\"556\" fill=\"#cbd5e1\" font-size=\"22\">top missed  {}</text>",
        xml_escape(&joined)
    )
}

/// Minimal XML text escaping for data-derived strings in the SVG card.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{analyze_history, render_before_card};

    fn history() -> Vec<String> {
        vec![
            "git status".into(),
            "git diff".into(),
            "cargo build".into(),
            "cargo test".into(),
            "lean-ctx gain".into(),
            "vim notes.txt".into(),
        ]
    }

    #[test]
    fn before_card_is_well_formed_and_branded() {
        let result = analyze_history(&history(), 20);
        let svg = render_before_card(&result);
        assert!(svg.starts_with("<svg"), "must be an SVG document");
        assert!(svg.trim_end().ends_with("</svg>"), "must close the svg tag");
        assert!(svg.contains("leanctx.com"), "must carry the brand footer");
        assert!(svg.contains("Ghost Tokens"), "must frame the leak");
        assert!(
            svg.contains("tokens/month left on the table"),
            "headline label present"
        );
    }

    #[test]
    fn xml_escape_neutralizes_markup() {
        assert_eq!(super::xml_escape("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&apos;");
    }
}
