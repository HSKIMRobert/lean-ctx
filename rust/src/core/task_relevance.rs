use std::collections::{HashMap, HashSet, VecDeque};

use super::graph_index::ProjectIndex;

#[derive(Debug, Clone)]
pub struct RelevanceScore {
    pub path: String,
    pub score: f64,
    pub recommended_mode: &'static str,
}

pub fn compute_relevance(
    index: &ProjectIndex,
    task_files: &[String],
    task_keywords: &[String],
) -> Vec<RelevanceScore> {
    let mut scores: HashMap<String, f64> = HashMap::new();

    // Seed: task files get score 1.0
    for f in task_files {
        scores.insert(f.clone(), 1.0);
    }

    // BFS from task files through import graph, decaying by distance
    let adj = build_adjacency(index);
    for seed in task_files {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        queue.push_back((seed.clone(), 0));
        visited.insert(seed.clone());

        while let Some((node, depth)) = queue.pop_front() {
            if depth > 4 {
                continue;
            }
            let decay = 1.0 / (1.0 + depth as f64).powi(2); // quadratic decay
            let entry = scores.entry(node.clone()).or_insert(0.0);
            *entry = entry.max(decay);

            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back((neighbor.clone(), depth + 1));
                    }
                }
            }
        }
    }

    // Keyword boost: files containing task keywords get a relevance boost
    if !task_keywords.is_empty() {
        let kw_lower: Vec<String> = task_keywords.iter().map(|k| k.to_lowercase()).collect();
        for (file_path, file_entry) in &index.files {
            let path_lower = file_path.to_lowercase();
            let mut keyword_hits = 0;
            for kw in &kw_lower {
                if path_lower.contains(kw) {
                    keyword_hits += 1;
                }
                for export in &file_entry.exports {
                    if export.to_lowercase().contains(kw) {
                        keyword_hits += 1;
                    }
                }
            }
            if keyword_hits > 0 {
                let boost = (keyword_hits as f64 * 0.15).min(0.6);
                let entry = scores.entry(file_path.clone()).or_insert(0.0);
                *entry = (*entry + boost).min(1.0);
            }
        }
    }

    let mut result: Vec<RelevanceScore> = scores
        .into_iter()
        .map(|(path, score)| {
            let mode = recommend_mode(score);
            RelevanceScore {
                path,
                score,
                recommended_mode: mode,
            }
        })
        .collect();

    result.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

fn recommend_mode(score: f64) -> &'static str {
    if score >= 0.8 {
        "full"
    } else if score >= 0.5 {
        "signatures"
    } else if score >= 0.2 {
        "map"
    } else {
        "reference"
    }
}

fn build_adjacency(index: &ProjectIndex) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &index.edges {
        adj.entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        adj.entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }
    adj
}

/// Extract likely task-relevant file paths and keywords from a task description.
pub fn parse_task_hints(task_description: &str) -> (Vec<String>, Vec<String>) {
    let mut files = Vec::new();
    let mut keywords = Vec::new();

    for word in task_description.split_whitespace() {
        let clean = word.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-'
        });
        if clean.contains('.')
            && (clean.contains('/')
                || clean.ends_with(".rs")
                || clean.ends_with(".ts")
                || clean.ends_with(".py")
                || clean.ends_with(".go")
                || clean.ends_with(".js"))
        {
            files.push(clean.to_string());
        } else if clean.len() >= 3 && !STOP_WORDS.contains(&clean.to_lowercase().as_str()) {
            keywords.push(clean.to_string());
        }
    }

    (files, keywords)
}

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "that", "this", "with", "from", "have", "has", "was", "are", "been",
    "not", "but", "all", "can", "had", "her", "one", "our", "out", "you", "its", "will", "each",
    "make", "like", "fix", "add", "use", "get", "set", "run", "new", "old", "should", "would",
    "could", "into", "also", "than", "them", "then", "when", "just", "only", "very", "some",
    "more", "other", "nach", "und", "die", "der", "das", "ist", "ein", "eine", "nicht", "auf",
    "mit",
];

/// Compute the Information Bottleneck approximation for a given content.
/// Uses task relevance + line entropy to score each line.
/// Returns lines sorted by information value, with a cutoff at the given budget.
pub fn information_bottleneck_filter(
    content: &str,
    task_keywords: &[String],
    budget_ratio: f64,
) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let kw_lower: Vec<String> = task_keywords.iter().map(|k| k.to_lowercase()).collect();

    let mut scored_lines: Vec<(usize, &str, f64)> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return (i, *line, 0.1); // keep some blank lines for structure
            }

            // Relevance: keyword hits in line
            let line_lower = trimmed.to_lowercase();
            let relevance: f64 = kw_lower
                .iter()
                .filter(|kw| line_lower.contains(kw.as_str()))
                .count() as f64;

            // Structural importance: definitions, returns, errors
            let structural = if is_definition_line(trimmed) {
                0.8
            } else if is_control_flow(trimmed) {
                0.4
            } else if is_closing_brace(trimmed) {
                0.2
            } else {
                0.3
            };

            // U-shaped position weight: edges of file are more important than middle
            let pos = i as f64 / lines.len().max(1) as f64;
            let u_weight = 0.6 + 0.4 * (4.0 * (pos - 0.5).powi(2)).min(1.0);

            let score = (relevance * 0.5 + structural + u_weight * 0.3).min(3.0);
            (i, *line, score)
        })
        .collect();

    // Keep lines above the budget ratio threshold
    let total_lines = scored_lines.len();
    let budget = ((total_lines as f64) * budget_ratio).ceil() as usize;

    // Sort by score descending to find cutoff
    let mut by_score = scored_lines.clone();
    by_score.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let cutoff_score = if budget < by_score.len() {
        by_score[budget].2
    } else {
        0.0
    };

    // Keep lines above cutoff, preserving original order
    scored_lines.retain(|(_, _, score)| *score >= cutoff_score);

    scored_lines
        .iter()
        .map(|(_, line, _)| *line)
        .collect::<Vec<&str>>()
        .join("\n")
}

fn is_definition_line(line: &str) -> bool {
    let prefixes = [
        "fn ",
        "pub fn ",
        "async fn ",
        "pub async fn ",
        "struct ",
        "pub struct ",
        "enum ",
        "pub enum ",
        "trait ",
        "pub trait ",
        "impl ",
        "type ",
        "pub type ",
        "const ",
        "pub const ",
        "static ",
        "pub static ",
        "class ",
        "export class ",
        "interface ",
        "export interface ",
        "function ",
        "export function ",
        "async function ",
        "def ",
        "async def ",
        "func ",
    ];
    prefixes
        .iter()
        .any(|p| line.starts_with(p) || line.trim_start().starts_with(p))
}

fn is_control_flow(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("if ")
        || trimmed.starts_with("else ")
        || trimmed.starts_with("match ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("break")
        || trimmed.starts_with("continue")
        || trimmed.starts_with("yield")
        || trimmed.starts_with("await ")
}

fn is_closing_brace(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "}" || trimmed == "};" || trimmed == "})" || trimmed == "});"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_task_finds_files_and_keywords() {
        let (files, keywords) =
            parse_task_hints("Fix the authentication bug in src/auth.rs and update tests");
        assert!(files.iter().any(|f| f.contains("auth.rs")));
        assert!(keywords
            .iter()
            .any(|k| k.to_lowercase().contains("authentication")));
    }

    #[test]
    fn recommend_mode_by_score() {
        assert_eq!(recommend_mode(1.0), "full");
        assert_eq!(recommend_mode(0.6), "signatures");
        assert_eq!(recommend_mode(0.3), "map");
        assert_eq!(recommend_mode(0.1), "reference");
    }

    #[test]
    fn info_bottleneck_preserves_definitions() {
        let content = "fn main() {\n    let x = 42;\n    // boring comment\n    println!(x);\n}\n";
        let result = information_bottleneck_filter(content, &["main".to_string()], 0.6);
        assert!(result.contains("fn main"));
    }
}
