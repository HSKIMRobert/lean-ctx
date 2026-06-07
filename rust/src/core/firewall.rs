//! Context firewall: replace large tool outputs with a compact digest + retrieval ref.
//!
//! When ephemeral mode is active (`[archive].ephemeral`, default on), genuinely large
//! tool results are stored out-of-band via [`crate::core::archive`] and only a
//! deterministic digest — a head/tail excerpt, size stats, and `ctx_expand` drilldown
//! instructions — is returned inline. This keeps the agent's context window small while
//! preserving full, slice-addressable access to the raw output.
//!
//! Scope: tool *outputs* (`ctx_shell`, `ctx_execute`, `ctx_search`, `ctx_tree`). Explicit
//! file reads keep their own read-mode system and are never firewalled.

use crate::core::config::Config;

const HEAD_LINES: usize = 20;
const TAIL_LINES: usize = 8;
const LONG_LINE_HEAD_CHARS: usize = 800;
const LONG_LINE_TAIL_CHARS: usize = 300;

/// Tools whose large outputs are eligible for the firewall. Explicit file reads are
/// intentionally excluded — they have their own read-mode (`lines:`, `signatures`, …).
pub fn is_firewallable_tool(name: &str) -> bool {
    matches!(
        name,
        "ctx_shell" | "ctx_execute" | "ctx_search" | "ctx_tree"
    )
}

/// Effective minimum token count before firewalling (config + env override).
pub fn min_tokens(config: &Config) -> usize {
    config.archive.ephemeral_min_tokens_effective()
}

/// Whether a result of `output_tokens` from `tool` should be firewalled.
pub fn should_firewall(tool: &str, output_tokens: usize, config: &Config) -> bool {
    config.archive.ephemeral_effective()
        && is_firewallable_tool(tool)
        && output_tokens >= min_tokens(config)
}

/// Build the inline digest that replaces a firewalled output. Deterministic (no LLM):
/// a head/tail excerpt for multi-line output, or a char-bounded excerpt for output with
/// few but very long lines (e.g. a single giant JSON line), followed by drilldown
/// instructions keyed on `archive_id`.
pub fn summarize(full: &str, archive_id: &str, tool: &str, output_tokens: usize) -> String {
    let chars = full.len();
    let lines: Vec<&str> = full.lines().collect();
    let line_count = lines.len();

    let mut out = String::new();
    out.push_str(&format!(
        "[Firewalled {tool} output — {chars} chars, {output_tokens} tok, {line_count} lines stored out-of-band]\n"
    ));

    if line_count > HEAD_LINES + TAIL_LINES + 1 {
        out.push_str("--- head ---\n");
        out.push_str(&lines[..HEAD_LINES].join("\n"));
        out.push_str(&format!(
            "\n--- … {} lines omitted … ---\n",
            line_count - HEAD_LINES - TAIL_LINES
        ));
        out.push_str("--- tail ---\n");
        out.push_str(&lines[line_count - TAIL_LINES..].join("\n"));
        out.push('\n');
    } else {
        // Few lines but large (e.g. one giant minified JSON line): char-bounded excerpt.
        let head_end = full.floor_char_boundary(LONG_LINE_HEAD_CHARS.min(chars));
        out.push_str(&full[..head_end]);
        if chars > LONG_LINE_HEAD_CHARS + LONG_LINE_TAIL_CHARS {
            out.push_str("\n… (truncated) …\n");
            let tail_start = full.floor_char_boundary(chars - LONG_LINE_TAIL_CHARS);
            out.push_str(&full[tail_start..]);
            out.push('\n');
        }
    }

    out.push_str("--- retrieve full output ---\n");
    out.push_str(&format!("Full:    ctx_expand(id=\"{archive_id}\")\n"));
    out.push_str(&format!(
        "Range:   ctx_expand(id=\"{archive_id}\", start_line=1, end_line=80)\n"
    ));
    out.push_str(&format!(
        "Head:    ctx_expand(id=\"{archive_id}\", head=120)\n"
    ));
    out.push_str(&format!(
        "Search:  ctx_expand(id=\"{archive_id}\", search=\"ERROR\")\n"
    ));
    out.push_str(&format!(
        "JSON:    ctx_expand(id=\"{archive_id}\", json_keys=true)"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firewallable_tools_are_outputs_not_reads() {
        assert!(is_firewallable_tool("ctx_shell"));
        assert!(is_firewallable_tool("ctx_search"));
        assert!(is_firewallable_tool("ctx_tree"));
        assert!(is_firewallable_tool("ctx_execute"));
        assert!(!is_firewallable_tool("ctx_read"));
        assert!(!is_firewallable_tool("ctx_multi_read"));
        assert!(!is_firewallable_tool("ctx_knowledge"));
    }

    #[test]
    fn should_firewall_respects_tool_and_threshold() {
        let mut cfg = Config::default();
        cfg.archive.enabled = true;
        cfg.archive.ephemeral = true;
        cfg.archive.ephemeral_min_tokens = 2000;
        // Env can override ephemeral; clear it for a deterministic test.
        std::env::remove_var("LEAN_CTX_EPHEMERAL");
        std::env::remove_var("LEAN_CTX_EPHEMERAL_MIN_TOKENS");

        assert!(should_firewall("ctx_shell", 5000, &cfg));
        assert!(!should_firewall("ctx_shell", 1000, &cfg)); // below threshold
        assert!(!should_firewall("ctx_read", 5000, &cfg)); // not firewallable
    }

    #[test]
    fn summarize_includes_excerpt_stats_and_ref() {
        let full = (1..=200)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let digest = summarize(&full, "abc123", "ctx_shell", 1234);
        assert!(digest.contains("Firewalled ctx_shell output"));
        assert!(digest.contains("1234 tok"));
        assert!(digest.contains("line 1")); // head
        assert!(digest.contains("line 200")); // tail
        assert!(digest.contains("lines omitted"));
        assert!(digest.contains("ctx_expand(id=\"abc123\")"));
        assert!(digest.contains("json_keys=true"));
        // The digest must be far smaller than the original.
        assert!(digest.len() < full.len());
    }

    #[test]
    fn summarize_handles_single_giant_line() {
        let full = "x".repeat(5000);
        let digest = summarize(&full, "id9", "ctx_search", 1300);
        assert!(digest.contains("Firewalled ctx_search output"));
        assert!(digest.contains("truncated"));
        assert!(digest.len() < full.len());
    }
}
