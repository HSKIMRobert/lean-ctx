use std::collections::{HashMap, HashSet};

use super::tokens::count_tokens;

#[derive(Debug)]
#[allow(dead_code)]
pub struct EntropyResult {
    pub output: String,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub techniques: Vec<String>,
}

#[allow(dead_code)]
impl EntropyResult {
    pub fn savings_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        let saved = self.original_tokens.saturating_sub(self.compressed_tokens);
        (saved as f64 / self.original_tokens as f64) * 100.0
    }
}

pub fn shannon_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let mut freq: HashMap<char, usize> = HashMap::new();
    let total = text.chars().count();

    for c in text.chars() {
        *freq.entry(c).or_default() += 1;
    }

    freq.values().fold(0.0_f64, |acc, &count| {
        let p = count as f64 / total as f64;
        acc - p * p.log2()
    })
}

pub fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

pub fn entropy_compress(content: &str) -> EntropyResult {
    let original_tokens = count_tokens(content);
    let mut lines: Vec<&str> = content.lines().collect();
    let mut techniques = Vec::new();

    let original_count = lines.len();
    lines.retain(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() < 3 {
            return true;
        }
        shannon_entropy(trimmed) >= 2.0
    });
    let removed = original_count - lines.len();
    if removed > 0 {
        techniques.push(format!("⊘ {removed} low-entropy lines (H<2.0)"));
    }

    let blocks = extract_blocks(&lines);
    let groups = find_pattern_groups(&blocks, 0.7);
    let mut dedup_count = 0;
    for group in &groups {
        if group.len() > 1 {
            dedup_count += group.len() - 1;
        }
    }
    if dedup_count > 0 {
        techniques.push(format!("⊘ {dedup_count} duplicate patterns (J≥0.7)"));
    }

    let mut result: Vec<String> = Vec::new();
    let mut skip_indices: HashSet<usize> = HashSet::new();
    for group in &groups {
        if group.len() > 1 {
            for &idx in &group[1..] {
                skip_indices.insert(idx);
            }
        }
    }
    for (i, line) in lines.iter().enumerate() {
        if !skip_indices.contains(&i) {
            result.push(line.to_string());
        }
    }

    let mut collapsed = Vec::new();
    let mut blank_count = 0;
    for line in &result {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                collapsed.push(line.clone());
            }
        } else {
            blank_count = 0;
            collapsed.push(line.clone());
        }
    }

    let output = collapsed.join("\n");
    let compressed_tokens = count_tokens(&output);

    EntropyResult {
        output,
        original_tokens,
        compressed_tokens,
        techniques,
    }
}

#[derive(Debug)]
pub struct EntropyAnalysis {
    pub avg_entropy: f64,
    pub low_entropy_count: usize,
    pub high_entropy_count: usize,
    pub total_lines: usize,
}

pub fn analyze_entropy(content: &str) -> EntropyAnalysis {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut sum = 0.0;
    let mut low = 0;
    let mut high = 0;
    let mut counted = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let h = shannon_entropy(trimmed);
        sum += h;
        counted += 1;
        if h < 2.0 {
            low += 1;
        }
        if h > 4.0 {
            high += 1;
        }
    }

    EntropyAnalysis {
        avg_entropy: if counted > 0 {
            sum / counted as f64
        } else {
            0.0
        },
        low_entropy_count: low,
        high_entropy_count: high,
        total_lines: total,
    }
}

#[allow(dead_code)]
struct Block {
    start: usize,
    content: String,
}

fn extract_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() && !current.is_empty() {
            blocks.push(Block {
                start,
                content: current.clone(),
            });
            current.clear();
        } else if !trimmed.is_empty() {
            if current.is_empty() {
                start = i;
            }
            current.push_str(trimmed);
            current.push('\n');
        }
    }

    if !current.is_empty() {
        blocks.push(Block {
            start,
            content: current,
        });
    }

    blocks
}

fn find_pattern_groups(blocks: &[Block], threshold: f64) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut assigned: HashSet<usize> = HashSet::new();

    for (i, block_a) in blocks.iter().enumerate() {
        if assigned.contains(&i) {
            continue;
        }
        let mut group = vec![i];
        for (j, block_b) in blocks.iter().enumerate().skip(i + 1) {
            if assigned.contains(&j) {
                continue;
            }
            if jaccard_similarity(&block_a.content, &block_b.content) >= threshold {
                group.push(j);
                assigned.insert(j);
            }
        }
        if group.len() > 1 {
            assigned.insert(i);
        }
        groups.push(group);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shannon_entropy_empty_is_zero() {
        assert_eq!(shannon_entropy(""), 0.0);
    }

    #[test]
    fn shannon_entropy_single_char() {
        assert_eq!(shannon_entropy("aaaa"), 0.0);
    }

    #[test]
    fn shannon_entropy_high_for_varied_text() {
        let varied = "abcdefghijklmnopqrstuvwxyz0123456789";
        let uniform = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert!(
            shannon_entropy(varied) > shannon_entropy(uniform),
            "varied text should have higher entropy"
        );
    }

    #[test]
    fn jaccard_identical_is_one() {
        let sim = jaccard_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_disjoint_is_zero() {
        let sim = jaccard_similarity("abc", "xyz");
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn jaccard_partial_overlap() {
        let sim = jaccard_similarity("hello world", "hello rust");
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn entropy_compress_produces_output() {
        let content = "fn main() {\n    println!(\"hello\");\n}\n\n// comment\n// another comment\n\nfn helper() {\n    let x = 42;\n}\n";
        let result = entropy_compress(content);
        assert!(!result.output.is_empty(), "should produce non-empty output");
        assert!(result.compressed_tokens <= result.original_tokens);
    }

    #[test]
    fn entropy_result_savings() {
        let r = EntropyResult {
            output: "short".to_string(),
            original_tokens: 100,
            compressed_tokens: 60,
            techniques: vec!["test".to_string()],
        };
        assert!((r.savings_percent() - 40.0).abs() < 0.1);
    }

    #[test]
    fn entropy_result_zero_original() {
        let r = EntropyResult {
            output: String::new(),
            original_tokens: 0,
            compressed_tokens: 0,
            techniques: vec![],
        };
        assert_eq!(r.savings_percent(), 0.0);
    }
}
