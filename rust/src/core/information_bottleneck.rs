//! QUITO-X–style trade-off: compress by dropping low token-entropy lines while targeting an output/input token ratio.

use super::entropy::normalized_token_entropy;
use super::tokens::count_tokens;

fn flush_omitted(out: &mut Vec<String>, run: &mut usize) {
    if *run > 0 {
        out.push(format!("// ... {} low-info lines omitted", *run));
        *run = 0;
    }
}

fn render_ib(lines: &[&str], scores: &[f64], threshold: f64) -> String {
    debug_assert_eq!(lines.len(), scores.len());
    let mut out = Vec::new();
    let mut omit_run = 0usize;
    for (&line, &score) in lines.iter().zip(scores.iter()) {
        if score >= threshold {
            flush_omitted(&mut out, &mut omit_run);
            out.push(line.to_string());
        } else {
            omit_run += 1;
        }
    }
    flush_omitted(&mut out, &mut omit_run);
    out.join("\n")
}

/// Compress `text` toward `target_ratio` (output tokens / input tokens) by dropping lines whose
/// normalized BPE token entropy falls below a dynamically chosen threshold.
pub fn compress_ib(text: &str, target_ratio: f64) -> String {
    if text.is_empty() {
        return String::new();
    }
    let input_tokens = count_tokens(text);
    if input_tokens == 0 {
        return text.to_string();
    }
    let ratio_target = target_ratio.clamp(0.02, 1.0);

    let lines_vec: Vec<&str> = text.lines().collect();
    let lines: &[&str] = &lines_vec;
    let scores: Vec<f64> = lines
        .iter()
        .map(|ln| normalized_token_entropy(ln))
        .collect();

    // Higher threshold ⇒ fewer kept lines ⇒ lower output ratio (monotone decreasing in threshold).
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    let mut best = render_ib(lines, &scores, 0.0);
    let mut best_diff = f64::INFINITY;

    let mut consider = |thr: f64| {
        let cand = render_ib(lines, &scores, thr);
        let r = count_tokens(&cand) as f64 / input_tokens as f64;
        let diff = (r - ratio_target).abs();
        if diff < best_diff {
            best_diff = diff;
            best = cand;
        }
    };

    for _ in 0..26 {
        let mid = (lo + hi) * 0.5;
        let cand = render_ib(lines, &scores, mid);
        let r = count_tokens(&cand) as f64 / input_tokens as f64;
        consider(mid);
        if r > ratio_target {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    for thr in [0.0_f64, 1.0_f64, lo, hi, (lo + hi) * 0.5] {
        consider(thr);
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_ratio_one_keeps_content() {
        assert_eq!(compress_ib("", 0.5), "");
        let s = "fn main() {\n    println!(\"hi\");\n}\n";
        let full = compress_ib(s, 1.0);
        assert!(full.contains("fn main"));
    }

    #[test]
    fn strong_compression_drops_redundant_lines() {
        let mut boring = String::new();
        for _ in 0..30 {
            boring.push_str("aaa bbb aaa bbb\n");
        }
        boring.push_str("unique_identifier_xyz_quartz\n");
        let out = compress_ib(&boring, 0.15);
        assert!(out.contains("low-info lines omitted"));
        assert!(out.contains("unique_identifier_xyz_quartz"));
        assert!(count_tokens(&out) < count_tokens(&boring));
    }

    #[test]
    fn placeholder_counts_skipped_lines() {
        let lines: Vec<String> = (0..5).map(|_| "x x x x".into()).collect();
        let mut text = lines.join("\n");
        text.push('\n');
        text.push_str("serde Deserialize TraitBounds\n");
        let out = compress_ib(&text, 0.25);
        assert!(out.contains("low-info lines omitted"));
        assert!(out.contains("serde"));
    }
}
