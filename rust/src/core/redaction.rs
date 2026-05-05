macro_rules! static_regex {
    ($pattern:expr) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| {
            regex::Regex::new($pattern).expect(concat!("BUG: invalid static regex: ", $pattern))
        })
    }};
}

pub fn redaction_enabled_for_active_role() -> bool {
    let role = crate::core::roles::active_role();
    if role.role.name == "admin" {
        role.io.redact_outputs
    } else {
        // Contract: redaction never disabled for non-admin roles.
        true
    }
}

pub fn redact_text_if_enabled(input: &str) -> String {
    if !redaction_enabled_for_active_role() {
        return input.to_string();
    }
    redact_text(input)
}

pub fn redact_text(input: &str) -> String {
    let patterns: Vec<(&str, &regex::Regex)> = vec![
        (
            "Bearer token",
            static_regex!(r"(?i)(bearer\s+)[a-zA-Z0-9\-_\.]{8,}"),
        ),
        (
            "Authorization header",
            static_regex!(r"(?i)(authorization:\s*(?:basic|bearer|token)\s+)[^\s\r\n]+"),
        ),
        (
            "API key param",
            static_regex!(
                r#"(?i)((?:api[_-]?key|apikey|access[_-]?key|secret[_-]?key|token|password|passwd|pwd|secret)\s*[=:]\s*)[^\s\r\n,;&"']+"#
            ),
        ),
        ("AWS key", static_regex!(r"(AKIA[0-9A-Z]{12,})")),
        (
            "Private key block",
            static_regex!(
                r"(?s)(-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----).+?(-----END\s+(?:RSA\s+)?PRIVATE\s+KEY-----)"
            ),
        ),
        (
            "GitHub token",
            static_regex!(r"(gh[pousr]_)[a-zA-Z0-9]{20,}"),
        ),
        (
            "Generic long secret",
            static_regex!(
                r#"(?i)(?:key|token|secret|password|credential|auth)\s*[=:]\s*['"]?([a-zA-Z0-9+/=\-_]{32,})['"]?"#
            ),
        ),
    ];

    let mut out = input.to_string();
    for (label, re) in &patterns {
        out = re
            .replace_all(&out, |caps: &regex::Captures| {
                if let Some(prefix) = caps.get(1) {
                    format!("{}[REDACTED:{}]", prefix.as_str(), label)
                } else {
                    format!("[REDACTED:{label}]")
                }
            })
            .to_string();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_token() {
        let s = "Authorization: Bearer abcdefghijklmnopqrstuvwxyz012345";
        let out = redact_text(s);
        assert!(out.contains("[REDACTED"));
        assert!(!out.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn redacts_private_key_block() {
        let s = "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----";
        let out = redact_text(s);
        assert!(out.contains("[REDACTED"));
        assert!(!out.contains("\nabc\n"));
    }
}
