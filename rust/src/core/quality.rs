#![allow(dead_code)]
use crate::core::preservation;

const QUALITY_THRESHOLD: f64 = 0.95;

#[derive(Debug, Clone)]
pub struct QualityScore {
    pub ast_score: f64,
    pub identifier_score: f64,
    pub line_score: f64,
    pub composite: f64,
    pub passed: bool,
}

impl QualityScore {
    pub fn format_compact(&self) -> String {
        if self.passed {
            format!(
                "Q:{:.0}% (ast:{:.0} id:{:.0} ln:{:.0}) ✓",
                self.composite * 100.0,
                self.ast_score * 100.0,
                self.identifier_score * 100.0,
                self.line_score * 100.0,
            )
        } else {
            format!(
                "Q:{:.0}% (ast:{:.0} id:{:.0} ln:{:.0}) ✗ BELOW THRESHOLD",
                self.composite * 100.0,
                self.ast_score * 100.0,
                self.identifier_score * 100.0,
                self.line_score * 100.0,
            )
        }
    }
}

pub fn score(original: &str, compressed: &str, ext: &str) -> QualityScore {
    let pres = preservation::measure(original, compressed, ext);
    let ast_score = pres.overall();

    let identifier_score = measure_identifier_preservation(original, compressed);
    let line_score = measure_line_preservation(original, compressed);

    // Weighted composite: AST is most critical, identifiers next, lines least
    let composite = ast_score * 0.5 + identifier_score * 0.3 + line_score * 0.2;
    let passed = composite >= QUALITY_THRESHOLD;

    QualityScore {
        ast_score,
        identifier_score,
        line_score,
        composite,
        passed,
    }
}

/// Guard: returns compressed if quality passes, original otherwise
pub fn guard(original: &str, compressed: &str, ext: &str) -> (String, QualityScore) {
    let q = score(original, compressed, ext);
    if q.passed {
        (compressed.to_string(), q)
    } else {
        (original.to_string(), q)
    }
}

fn measure_identifier_preservation(original: &str, compressed: &str) -> f64 {
    let ident_re = regex::Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]{3,}\b").unwrap();

    let original_idents: std::collections::HashSet<&str> =
        ident_re.find_iter(original).map(|m| m.as_str()).collect();

    if original_idents.is_empty() {
        return 1.0;
    }

    let preserved = original_idents
        .iter()
        .filter(|id| compressed.contains(*id))
        .count();

    preserved as f64 / original_idents.len() as f64
}

fn measure_line_preservation(original: &str, compressed: &str) -> f64 {
    let original_lines: usize = original.lines().filter(|l| !l.trim().is_empty()).count();
    if original_lines == 0 {
        return 1.0;
    }

    let compressed_lines: usize = compressed.lines().filter(|l| !l.trim().is_empty()).count();
    let ratio = compressed_lines as f64 / original_lines as f64;

    ratio.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score_identity() {
        let code = "fn main() {\n    println!(\"hello\");\n}\n";
        let q = score(code, code, "rs");
        assert!(q.composite >= 0.99);
        assert!(q.passed);
    }

    #[test]
    fn test_score_below_threshold_returns_original() {
        let original = "fn validate_token() {\n    let result = check();\n    return result;\n}\n";
        let bad_compressed = "removed everything";
        let (output, q) = guard(original, bad_compressed, "rs");
        assert!(!q.passed);
        assert_eq!(output, original);
    }

    #[test]
    fn test_good_compression_passes() {
        let original = "fn validate_token() {\n    let result = check();\n    return result;\n}\n";
        let compressed = "fn validate_token() { let result = check(); return result; }";
        let q = score(original, compressed, "rs");
        assert!(q.ast_score >= 0.9);
        assert!(q.identifier_score >= 0.9);
    }

    #[test]
    fn test_score_format_compact() {
        let code = "fn main() {}\n";
        let q = score(code, code, "rs");
        let formatted = q.format_compact();
        assert!(formatted.contains("Q:"));
        assert!(formatted.contains("✓"));
    }

    #[test]
    fn test_empty_content_scores_perfect() {
        let q = score("", "", "rs");
        assert!(q.passed);
        assert!(q.composite >= 0.99);
    }

    #[test]
    fn test_rust_file_with_structs() {
        let original = "pub struct Config {\n    pub name: String,\n    pub value: usize,\n}\n\nimpl Config {\n    pub fn new() -> Self {\n        Self { name: String::new(), value: 0 }\n    }\n}\n";
        let compressed = "pub struct Config { pub name: String, pub value: usize }\nimpl Config { pub fn new() -> Self { Self { name: String::new(), value: 0 } } }";
        let q = score(original, compressed, "rs");
        assert!(q.identifier_score >= 0.9);
    }

    #[test]
    fn test_typescript_file() {
        let original = "export function fetchData(url: string): Promise<Response> {\n  return fetch(url);\n}\n\nexport const API_URL = 'https://api.example.com';\n";
        let compressed = "export function fetchData(url: string): Promise<Response> { return fetch(url); }\nexport const API_URL = 'https://api.example.com';";
        let q = score(original, compressed, "ts");
        assert!(q.identifier_score >= 0.9);
    }

    #[test]
    fn test_python_file() {
        let original = "def validate_credentials(username: str, password: str) -> bool:\n    user = find_user(username)\n    return verify_hash(user.password_hash, password)\n";
        let compressed = "def validate_credentials(username, password): user = find_user(username); return verify_hash(user.password_hash, password)";
        let q = score(original, compressed, "py");
        assert!(q.identifier_score >= 0.8);
    }
}
