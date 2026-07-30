//! Turn-level context budget controller (#1306).
//!
//! Caps fresh tokens per tool response to prevent context window bloat.
//! Research basis: "Context Length Alone Hurts LLM Performance" (EMNLP 2025)
//! found 13.9–85% degradation with length even with perfect retrieval.

use super::tokens::count_tokens;

/// Budget enforcement result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BudgetAction {
    /// Content fits within budget — pass through unchanged.
    PassThrough,
    /// Content exceeds budget — truncated with expand hint appended.
    Truncated {
        original_tokens: usize,
        delivered_tokens: usize,
    },
}

/// Apply turn-level token budget to a tool response body.
///
/// If the response exceeds `fresh_limit` tokens, truncates to fit and appends
/// an expand hint so the agent can retrieve the remainder.
///
/// Returns `(possibly_truncated_text, action)`.
pub(crate) fn apply_turn_budget(text: &str, fresh_limit: usize) -> (String, BudgetAction) {
    if fresh_limit == 0 {
        return (text.to_string(), BudgetAction::PassThrough);
    }

    let token_count = count_tokens(text);
    if token_count <= fresh_limit {
        return (text.to_string(), BudgetAction::PassThrough);
    }

    let truncated = truncate_to_token_budget(text, fresh_limit);
    let delivered_tokens = count_tokens(&truncated);

    let hint = format!(
        "\n[… truncated at ~{delivered_tokens} of {token_count} tokens — \
         use ctx_read with lines= parameter to see specific sections]"
    );

    (
        format!("{truncated}{hint}"),
        BudgetAction::Truncated {
            original_tokens: token_count,
            delivered_tokens,
        },
    )
}

/// Truncate text to approximately `limit` tokens by keeping complete lines.
fn truncate_to_token_budget(text: &str, limit: usize) -> String {
    let mut result = String::new();
    let mut current_tokens = 0;

    for line in text.lines() {
        let line_tokens = count_tokens(line);
        if current_tokens + line_tokens > limit && current_tokens > 0 {
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
        current_tokens += line_tokens;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_within_budget() {
        let text = "small content";
        let (result, action) = apply_turn_budget(text, 1000);
        assert_eq!(result, text);
        assert_eq!(action, BudgetAction::PassThrough);
    }

    #[test]
    fn passthrough_when_budget_is_zero() {
        let text = "any content at all";
        let (result, action) = apply_turn_budget(text, 0);
        assert_eq!(result, text);
        assert_eq!(action, BudgetAction::PassThrough);
    }

    #[test]
    fn truncates_large_content() {
        let lines: Vec<String> = (0..200)
            .map(|i| format!("fn function_{i}() {{ let x = {i}; }}"))
            .collect();
        let text = lines.join("\n");
        let (result, action) = apply_turn_budget(&text, 100);

        assert!(result.contains("[… truncated"));
        assert!(result.contains("use ctx_read with lines="));
        match action {
            BudgetAction::Truncated {
                original_tokens,
                delivered_tokens,
            } => {
                assert!(
                    delivered_tokens <= 120,
                    "delivered {delivered_tokens} > ~120"
                );
                assert!(original_tokens > delivered_tokens);
            }
            BudgetAction::PassThrough => panic!("should have truncated"),
        }
    }

    #[test]
    fn truncation_preserves_complete_lines() {
        let text = "line one\nline two\nline three\nline four\nline five";
        let (result, _) = apply_turn_budget(text, 5);
        let body = result.split("\n[… truncated").next().unwrap();
        assert!(
            !body.ends_with(char::is_whitespace),
            "truncated body should end with a complete line"
        );
    }
}
