//! PII detection with checksum guardrails (GL #675).
//!
//! Each detector pairs a regex with a *validator* so structurally-valid noise
//! (a random 16-digit order number, a phone number that looks card-shaped) is
//! not flagged. Cards use the Luhn checksum, IBANs the ISO-7064 mod-97 check,
//! and Swiss AHV/AVS numbers the EAN-13 check digit. The result is a low
//! false-positive surface suitable for redacting content before it reaches the
//! model.
//!
//! Privacy: callers receive only `(class, count)` pairs — never the matched
//! value — so audit logs can record *that* PII was present without leaking it.

use std::sync::OnceLock;

use regex::{Captures, Regex};

/// One PII detector: a labelled regex plus a checksum/shape validator. A match
/// is only counted/redacted when `validate` accepts the exact matched text.
struct PiiRule {
    class: &'static str,
    re: Regex,
    validate: fn(&str) -> bool,
}

fn rules() -> &'static [PiiRule] {
    static RULES: OnceLock<Vec<PiiRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            // Swiss AHV/AVS social-security number (EAN-13, prefixed 756).
            PiiRule {
                class: "ch_ahv",
                re: Regex::new(r"\b756[.\s]?\d{4}[.\s]?\d{4}[.\s]?\d{2}\b").unwrap(),
                validate: ahv_valid,
            },
            // IBAN — run before the card rule so its digits aren't re-matched.
            PiiRule {
                class: "iban",
                re: Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]){11,30}\b").unwrap(),
                validate: iban_valid,
            },
            // Payment card (13–19 digits, optional space/hyphen groups), Luhn.
            PiiRule {
                class: "card",
                re: Regex::new(r"\b\d(?:[ -]?\d){12,18}\b").unwrap(),
                validate: luhn_valid,
            },
            // Email — specific enough that no extra validation is needed.
            PiiRule {
                class: "email",
                re: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap(),
                validate: |_| true,
            },
        ]
    })
}

/// Redact every validated PII match to `[REDACTED:<class>]`. Returns the
/// transformed text plus per-class hit counts (for privacy-preserving audit).
#[must_use]
pub fn redact(text: &str) -> (String, Vec<(&'static str, usize)>) {
    let mut out = text.to_string();
    let mut counts = Vec::new();
    for rule in rules() {
        let mut n = 0usize;
        out = rule
            .re
            .replace_all(&out, |caps: &Captures| {
                let m = caps.get(0).map_or("", |g| g.as_str());
                if (rule.validate)(m) {
                    n += 1;
                    format!("[REDACTED:{}]", rule.class)
                } else {
                    m.to_string()
                }
            })
            .to_string();
        if n > 0 {
            counts.push((rule.class, n));
        }
    }
    (out, counts)
}

/// Count validated PII matches per class without rewriting (for block
/// decisions). Empty = no PII detected.
#[must_use]
pub fn detect(text: &str) -> Vec<(&'static str, usize)> {
    let mut counts = Vec::new();
    for rule in rules() {
        let n = rule
            .re
            .find_iter(text)
            .filter(|m| (rule.validate)(m.as_str()))
            .count();
        if n > 0 {
            counts.push((rule.class, n));
        }
    }
    counts
}

fn digits(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

/// Luhn checksum for payment cards (13–19 digits).
fn luhn_valid(s: &str) -> bool {
    let d = digits(s);
    if d.len() < 13 || d.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    for &digit in d.iter().rev() {
        let mut x = digit;
        if double {
            x *= 2;
            if x > 9 {
                x -= 9;
            }
        }
        sum += x;
        double = !double;
    }
    sum.is_multiple_of(10)
}

/// EAN-13 check digit for a Swiss AHV number (must start 756, 13 digits).
fn ahv_valid(s: &str) -> bool {
    let d = digits(s);
    if d.len() != 13 || d[0] != 7 || d[1] != 5 || d[2] != 6 {
        return false;
    }
    let mut sum = 0u32;
    for (i, &digit) in d[..12].iter().enumerate() {
        sum += if i.is_multiple_of(2) {
            digit
        } else {
            digit * 3
        };
    }
    let check = (10 - (sum % 10)) % 10;
    check == d[12]
}

/// ISO-7064 mod-97 check for an IBAN.
fn iban_valid(s: &str) -> bool {
    let compact: String = s
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_uppercase();
    if compact.len() < 15 || compact.len() > 34 {
        return false;
    }
    let (head, tail) = compact.split_at(4);
    let mut remainder = 0u32;
    for c in tail.chars().chain(head.chars()) {
        if let Some(dval) = c.to_digit(10) {
            remainder = (remainder * 10 + dval) % 97;
        } else {
            // Letter → two-digit value (A=10 … Z=35).
            let v = (c as u32) - ('A' as u32) + 10;
            remainder = (remainder * 100 + v) % 97;
        }
    }
    remainder == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_valid_ch_ahv() {
        // 756.9217.0769.85 is a valid EAN-13 AHV test number.
        let (out, counts) = redact("AHV 756.9217.0769.85 on file");
        assert!(out.contains("[REDACTED:ch_ahv]"), "{out}");
        assert_eq!(counts, vec![("ch_ahv", 1)]);
    }

    #[test]
    fn ignores_ahv_with_bad_checksum() {
        let (out, counts) = redact("756.9217.0769.86 is not valid");
        assert!(
            out.contains("756.9217.0769.86"),
            "must not redact invalid AHV"
        );
        assert!(counts.is_empty());
    }

    #[test]
    fn redacts_valid_card_via_luhn() {
        // 4111 1111 1111 1111 is the canonical Luhn-valid Visa test number.
        let (out, _) = redact("card 4111 1111 1111 1111 expires");
        assert!(out.contains("[REDACTED:card]"), "{out}");
    }

    #[test]
    fn ignores_non_luhn_16_digits() {
        // A random 16-digit order id that fails Luhn must survive.
        let (out, counts) = redact("order 1234567890123456 shipped");
        assert!(out.contains("1234567890123456"), "false positive: {out}");
        assert!(counts.is_empty());
    }

    #[test]
    fn redacts_valid_iban() {
        // GB82 WEST 1234 5698 7654 32 is a valid mod-97 IBAN.
        let (out, _) = redact("pay to GB82 WEST 1234 5698 7654 32 today");
        assert!(out.contains("[REDACTED:iban]"), "{out}");
    }

    #[test]
    fn ignores_invalid_iban() {
        let (out, counts) = redact("ref GB00WEST12345698765432 here");
        assert!(out.contains("GB00WEST12345698765432"));
        assert!(counts.is_empty());
    }

    #[test]
    fn redacts_email() {
        let (out, _) = redact("contact jane.doe@example.com please");
        assert!(out.contains("[REDACTED:email]"), "{out}");
    }

    #[test]
    fn detect_counts_match_redact() {
        let text = "jane@example.com and 4111 1111 1111 1111";
        let counts = detect(text);
        assert!(counts.iter().any(|(c, _)| *c == "email"));
        assert!(counts.iter().any(|(c, _)| *c == "card"));
    }

    #[test]
    fn clean_text_has_no_pii() {
        assert!(detect("just some ordinary source code, no secrets").is_empty());
    }
}
