use crate::core::tokens::count_tokens;
use crate::core::web::distill;

/// Char budget for the research-prose squeeze (~6k tokens). Only oversized prose
/// is truncated; the squeeze's main job is dedup + blank-collapse, not cutting.
const RESEARCH_PROSE_CAP: usize = 24_000;

/// Proxy compression funnel: routes a tool result to the right compressor.
///
/// 1. Already-cited research output (from `ctx_url_read` / the web layer) is kept
///    verbatim — it is distilled and citation-stamped, so the shell pipeline must
///    not touch its footer or claim markers.
/// 2. Prose results (web fetches, doc reads, research MCP bridges) are squeezed
///    by the prose-aware research compressor instead of the log/code-tuned shell
///    engine.
/// 3. Everything else (shell/build/search output) flows through the unified
///    `compress_if_beneficial` pipeline. A `$ ...` command hint is extracted so
///    the pattern engine gets the same routing as the CLI and MCP paths.
pub fn compress_tool_result(content: &str, tool_name: Option<&str>) -> String {
    if content.trim().is_empty() || content.len() < 200 {
        return content.to_string();
    }

    if is_cited_research_output(content) {
        return content.to_string();
    }

    if extract_command_hint(content).is_none() && looks_like_prose(content) {
        if let Some(out) = squeeze_research_prose(content) {
            return out;
        }
    }

    let cmd = infer_command(content, tool_name);
    crate::shell::compress::engine::compress_if_beneficial(&cmd, content)
}

/// True when `content` is a lean-ctx web read: distilled body + citation footer
/// (`Source: …\nSite: … · Retrieved: …`). Such output is re-compression-hostile.
fn is_cited_research_output(content: &str) -> bool {
    content.contains("· Retrieved: ") && content.contains("\nSource: ")
}

/// Code/shell symbols whose density cleanly separates source/logs from prose.
const CODE_SYMBOLS: &str = "{}<>;=|\\$`";

/// Conservative prose detector: substantial, letter-dense, low code-symbol, with
/// real sentences and long lines. Code, logs, tables and JSON all fail this.
fn looks_like_prose(content: &str) -> bool {
    let sample: String = content.chars().take(4000).collect();
    let total = sample.chars().count();
    if total < 600 {
        return false;
    }
    let total_f = total as f32;
    let alpha = sample.chars().filter(|c| c.is_alphabetic()).count() as f32;
    let spaces = sample.chars().filter(|c| *c == ' ').count() as f32;
    let symbols = sample.chars().filter(|c| CODE_SYMBOLS.contains(*c)).count() as f32;

    if alpha / total_f < 0.6 || spaces / total_f < 0.12 || symbols / total_f > 0.06 {
        return false;
    }
    if sample.matches(['.', '!', '?']).count() < 4 {
        return false;
    }

    let non_empty: Vec<&str> = sample.lines().filter(|l| !l.trim().is_empty()).collect();
    if non_empty.is_empty() {
        return false;
    }
    let avg_len =
        non_empty.iter().map(|l| l.chars().count()).sum::<usize>() as f32 / non_empty.len() as f32;
    avg_len >= 40.0
}

/// Apply the prose squeeze, returning a footer-stamped result only when it
/// actually saves tokens; otherwise `None` so the normal pipeline can try.
fn squeeze_research_prose(content: &str) -> Option<String> {
    let before = count_tokens(content);
    let squeezed = distill::squeeze_prose(content, RESEARCH_PROSE_CAP);
    if squeezed.trim().is_empty() {
        return None;
    }
    let after = count_tokens(&squeezed);
    if after + 2 >= before {
        return None;
    }
    Some(crate::core::protocol::append_savings_with_info(
        &squeezed,
        before,
        after,
        Some("research"),
        None,
    ))
}

fn infer_command(content: &str, tool_name: Option<&str>) -> String {
    if let Some(cmd) = extract_command_hint(content) {
        return cmd;
    }

    if let Some(name) = tool_name {
        let nl = name.to_lowercase();
        if nl.contains("bash") || nl.contains("shell") || nl.contains("terminal") {
            return "shell".to_string();
        }
        if nl.contains("search") || nl.contains("grep") || nl.contains("find") {
            return "grep".to_string();
        }
    }

    String::new()
}

fn extract_command_hint(content: &str) -> Option<String> {
    for line in content.lines().take(3) {
        let trimmed = line.trim();
        if let Some(cmd) = trimmed.strip_prefix("$ ") {
            return Some(cmd.to_string());
        }
        if let Some(cmd) = trimmed.strip_prefix("% ") {
            return Some(cmd.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_content_unchanged() {
        let short = "hello world";
        assert_eq!(compress_tool_result(short, None), short);
    }

    #[test]
    fn empty_content_unchanged() {
        assert_eq!(compress_tool_result("", None), "");
        assert_eq!(compress_tool_result("   ", None), "   ");
    }

    #[test]
    fn command_hint_extraction() {
        assert_eq!(
            extract_command_hint("$ cargo build\nCompiling foo"),
            Some("cargo build".to_string())
        );
        assert_eq!(extract_command_hint("no prefix here"), None);
    }

    #[test]
    fn tool_name_inference() {
        assert_eq!(infer_command("some text", Some("bash_execute")), "shell");
        assert_eq!(infer_command("some text", Some("search_files")), "grep");
        assert_eq!(infer_command("some text", Some("unknown_tool")), "");
    }

    #[test]
    fn cited_research_output_is_preserved_verbatim() {
        let cited = format!(
            "Rust is a language.\n\n---\nSource: Rust — https://x.com/a\n\
             Site: x.com · Retrieved: 2026-06-06T00:00:00Z\n{}",
            "Extra body line that would otherwise be touched. ".repeat(20)
        );
        assert_eq!(compress_tool_result(&cited, Some("ctx_url_read")), cited);
    }

    #[test]
    fn prose_is_squeezed_and_deduped() {
        let para = "Rust is a multi-paradigm systems programming language that \
                    emphasizes performance, type safety, and fearless concurrency, \
                    achieving memory safety without a garbage collector at runtime.";
        // Repeated paragraph (well over the 600-char prose floor) → dedup keeps one.
        let input = format!("{}\n", [para; 8].join("\n\n"));
        assert!(input.len() > 600);
        let out = compress_tool_result(&input, Some("web_fetch"));
        assert_eq!(out.matches("fearless concurrency").count(), 1);
        assert!(out.contains("performance, type safety"));
    }

    #[test]
    fn code_output_is_not_treated_as_prose() {
        let code = "fn main() {\n    let x = vec![1, 2, 3];\n    \
                    for i in &x { println!(\"{}\", i); }\n}\n"
            .repeat(20);
        assert!(!looks_like_prose(&code));
    }

    #[test]
    fn shell_log_is_not_treated_as_prose() {
        let log = "$ cargo build\n   Compiling foo v0.1.0\n    Finished dev\n".repeat(20);
        assert!(!looks_like_prose(&log));
    }
}
