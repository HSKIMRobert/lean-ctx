use crate::core::protocol;

pub fn shell_savings_footer(output: &str, original: usize, compressed: usize) -> String {
    if !protocol::savings_footer_visible() {
        return output.to_string();
    }
    let saved = original.saturating_sub(compressed);
    if original == 0 || saved == 0 {
        return output.to_string();
    }
    let pct = (saved as f64 / original as f64 * 100.0).round() as usize;
    if pct < 5 {
        return output.to_string();
    }
    format!("{output}\n[lean-ctx: {original}→{compressed} tok, -{pct}%]")
}
