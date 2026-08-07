use crate::core::signatures::{Signature, extract_signatures};
use crate::core::tokens::count_tokens;

/// Maximum chunks to return (Miller's Law upper bound).
const MAX_CHUNKS: usize = 9;
/// Default chunks to return.
const DEFAULT_CHUNKS: usize = 7;

/// Semantic role of a source-code chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    /// A function or method implementation.
    Function,
    /// A type, trait, class, or implementation declaration.
    Type,
    /// A test function or test block.
    Test,
    /// Configuration content.
    Config,
    /// Comment-only content.
    Comment,
    /// A leading import group.
    Import,
    /// Source between recognized semantic boundaries.
    Block,
}

/// A bounded, scored section of source code.
#[derive(Debug, Clone)]
pub struct SemanticChunk {
    /// Source text contained in this chunk.
    pub content: String,
    /// Semantic role used for budget prioritization.
    pub kind: ChunkKind,
    /// Heuristic complexity derived from size, nesting, and branch count.
    pub complexity: f64,
    /// Inclusive, one-based source line range.
    pub line_range: (usize, usize),
    /// Model-correct token count for `content`.
    pub token_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct Boundary {
    start: usize,
    end: usize,
    kind: ChunkKind,
}

/// Detect semantic chunks in source code using tree-sitter signature boundaries.
pub(crate) fn detect_chunks(content: &str, file_ext: &str) -> Vec<SemanticChunk> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut boundaries = signature_boundaries(&extract_signatures(content, file_ext), &lines);
    if let Some(imports) = leading_import_boundary(&lines) {
        boundaries.push(imports);
    }
    boundaries.sort_by_key(|boundary| (boundary.start, boundary.end));

    let mut chunks = Vec::new();
    let mut cursor = 1;
    for (position, boundary) in boundaries.iter().enumerate() {
        if boundary.start > cursor {
            push_chunk(
                &mut chunks,
                &lines,
                cursor,
                boundary.start - 1,
                None,
                file_ext,
            );
        }
        if boundary.end < cursor {
            continue;
        }
        let next_start = boundaries
            .get(position + 1)
            .map_or(lines.len() + 1, |next| next.start);
        let end = boundary.end.min(next_start.saturating_sub(1));
        let start = boundary.start.max(cursor);
        push_chunk(
            &mut chunks,
            &lines,
            start,
            end,
            Some(boundary.kind),
            file_ext,
        );
        cursor = end.saturating_add(1);
    }
    if cursor <= lines.len() {
        push_chunk(&mut chunks, &lines, cursor, lines.len(), None, file_ext);
    }
    chunks
}

fn signature_boundaries(signatures: &[Signature], lines: &[&str]) -> Vec<Boundary> {
    let mut boundaries: Vec<Boundary> = signatures
        .iter()
        .filter_map(|signature| {
            let mut start = signature.start_line?;
            let end = signature.end_line?.min(lines.len());
            if start == 0 || start > end {
                return None;
            }
            start = include_rust_attributes(start, lines);
            let text = lines[start - 1..end].join("\n");
            Some(Boundary {
                start,
                end,
                kind: signature_kind(signature, &text),
            })
        })
        .collect();
    boundaries.sort_by_key(|boundary| (boundary.start, boundary.end));
    boundaries.dedup_by_key(|boundary| (boundary.start, boundary.end));
    boundaries
}

fn include_rust_attributes(mut start: usize, lines: &[&str]) -> usize {
    while start > 1 {
        let previous = lines[start - 2].trim();
        if previous.starts_with("#[") || previous.starts_with("///") {
            start -= 1;
        } else {
            break;
        }
    }
    start
}

fn signature_kind(signature: &Signature, text: &str) -> ChunkKind {
    if signature.name.starts_with("test_")
        || text.contains("#[test]")
        || text.contains("#[tokio::test]")
    {
        ChunkKind::Test
    } else if matches!(signature.kind, "fn" | "method" | "constructor") {
        ChunkKind::Function
    } else if matches!(
        signature.kind,
        "struct" | "enum" | "trait" | "type" | "class" | "interface" | "impl" | "record"
    ) {
        ChunkKind::Type
    } else {
        ChunkKind::Block
    }
}

fn leading_import_boundary(lines: &[&str]) -> Option<Boundary> {
    let start = lines.iter().position(|line| is_import_start(line))?;
    if lines[..start]
        .iter()
        .any(|line| !line.trim().is_empty() && !is_comment_line(line))
    {
        return None;
    }

    let mut end = start;
    let mut delimiter_depth = 0_isize;
    let mut continued = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim();
        if is_import_start(line) || delimiter_depth > 0 || continued || trimmed.is_empty() {
            end = index;
            delimiter_depth += delimiter_delta(trimmed);
            delimiter_depth = delimiter_depth.max(0);
            continued = trimmed.ends_with('\\');
        } else {
            break;
        }
    }
    Some(Boundary {
        start: start + 1,
        end: end + 1,
        kind: ChunkKind::Import,
    })
}

fn delimiter_delta(line: &str) -> isize {
    line.chars().fold(0, |depth, character| match character {
        '{' | '(' | '[' => depth + 1,
        '}' | ')' | ']' => depth - 1,
        _ => depth,
    })
}

fn is_import_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("use ")
        || trimmed.starts_with("pub use ")
        || trimmed.starts_with("extern crate ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("#include ")
}

fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.ends_with("*/")
}

fn push_chunk(
    chunks: &mut Vec<SemanticChunk>,
    lines: &[&str],
    start: usize,
    end: usize,
    explicit_kind: Option<ChunkKind>,
    file_ext: &str,
) {
    if start == 0 || end < start || start > lines.len() {
        return;
    }
    let content = lines[start - 1..end.min(lines.len())].join("\n");
    if content.trim().is_empty() {
        return;
    }
    let kind = explicit_kind.unwrap_or_else(|| gap_kind(&content, file_ext));
    let complexity = chunk_complexity(&content);
    let token_count = count_tokens(&content);
    chunks.push(SemanticChunk {
        content,
        kind,
        complexity,
        line_range: (start, end.min(lines.len())),
        token_count,
    });
}

fn gap_kind(content: &str, file_ext: &str) -> ChunkKind {
    if content
        .lines()
        .all(|line| line.trim().is_empty() || is_comment_line(line))
    {
        ChunkKind::Comment
    } else if matches!(file_ext, "json" | "toml" | "yaml" | "yml" | "ini") {
        ChunkKind::Config
    } else {
        ChunkKind::Block
    }
}

fn chunk_complexity(content: &str) -> f64 {
    let line_count = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let mut depth = 0_usize;
    let mut max_depth = 0_usize;
    for character in content.chars() {
        match character {
            '{' | '(' | '[' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            '}' | ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let branches = [" if ", " else ", " match ", " for ", " while ", "&&", "||"]
        .iter()
        .map(|needle| content.matches(needle).count())
        .sum::<usize>();
    1.0 + (line_count as f64).ln_1p() + max_depth as f64 * 0.5 + branches as f64
}

/// Select top-K chunks within the cognitive budget (default 7, max 9).
pub(crate) fn budget_select(chunks: &[SemanticChunk], max_chunks: Option<usize>) -> Vec<usize> {
    let limit = max_chunks.unwrap_or(DEFAULT_CHUNKS).min(MAX_CHUNKS);
    let mut ranked: Vec<usize> = (0..chunks.len()).collect();
    ranked.sort_by(|left, right| {
        let left_score = kind_priority(chunks[*left].kind) as f64 * chunks[*left].complexity;
        let right_score = kind_priority(chunks[*right].kind) as f64 * chunks[*right].complexity;
        right_score
            .total_cmp(&left_score)
            .then_with(|| chunks[*left].line_range.cmp(&chunks[*right].line_range))
    });
    ranked.truncate(limit);
    ranked.sort_by_key(|index| chunks[*index].line_range);
    ranked
}

fn kind_priority(kind: ChunkKind) -> u8 {
    match kind {
        ChunkKind::Test => 5,
        ChunkKind::Function => 4,
        ChunkKind::Type => 3,
        ChunkKind::Import | ChunkKind::Config => 2,
        ChunkKind::Block => 1,
        ChunkKind::Comment => 0,
    }
}

/// Render selected chunks as output text with inter-chunk markers.
pub(crate) fn render_budget_output(
    chunks: &[SemanticChunk],
    selected: &[usize],
    file_path: &str,
) -> String {
    let mut ordered: Vec<&SemanticChunk> = selected
        .iter()
        .filter_map(|index| chunks.get(*index))
        .collect();
    ordered.sort_by_key(|chunk| chunk.line_range);
    let total_tokens = ordered.iter().map(|chunk| chunk.token_count).sum::<usize>();
    let mut output = format!("// {file_path}\n");
    let mut previous_end = None;
    for chunk in &ordered {
        if let Some(end) = previous_end {
            let omitted = chunk.line_range.0.saturating_sub(end + 1);
            if omitted > 0 {
                output.push_str(&format!("// ... {omitted} lines omitted\n"));
            }
        }
        output.push_str(&format!(
            "§ {} {} (L{}-L{})\n{}\n",
            kind_label(chunk.kind),
            chunk_name(chunk),
            chunk.line_range.0,
            chunk.line_range.1,
            chunk.content
        ));
        previous_end = Some(chunk.line_range.1);
    }
    output.push_str(&format!(
        "{}/{} chunks shown ({} tokens)",
        ordered.len(),
        chunks.len(),
        total_tokens
    ));
    output
}

fn kind_label(kind: ChunkKind) -> &'static str {
    match kind {
        ChunkKind::Function => "function",
        ChunkKind::Type => "type",
        ChunkKind::Test => "test",
        ChunkKind::Config => "config",
        ChunkKind::Comment => "comment",
        ChunkKind::Import => "import",
        ChunkKind::Block => "block",
    }
}

fn chunk_name(chunk: &SemanticChunk) -> &str {
    if matches!(chunk.kind, ChunkKind::Import) {
        return "imports";
    }
    let words: Vec<&str> = chunk.content.split_whitespace().collect();
    let keyword = match chunk.kind {
        ChunkKind::Function | ChunkKind::Test => ["fn", "def", "function"].as_slice(),
        ChunkKind::Type => [
            "struct",
            "enum",
            "trait",
            "type",
            "class",
            "interface",
            "impl",
        ]
        .as_slice(),
        _ => return kind_label(chunk.kind),
    };
    words
        .windows(2)
        .find(|pair| keyword.contains(&pair[0]))
        .map_or(kind_label(chunk.kind), |pair| {
            pair[1].trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        })
}

#[cfg(test)]
mod tests {
    use super::{ChunkKind, SemanticChunk, budget_select, detect_chunks, render_budget_output};

    fn chunk(kind: ChunkKind, line: usize, complexity: f64) -> SemanticChunk {
        SemanticChunk {
            content: format!("line {line}"),
            kind,
            complexity,
            line_range: (line, line),
            token_count: 2,
        }
    }

    #[test]
    fn detect_chunks_finds_functions_in_rust() {
        let source = "fn one() {}\n\nfn two() { if true {} }\n\nfn three() {}\n";
        let chunks = detect_chunks(source, "rs");
        assert_eq!(
            chunks
                .iter()
                .filter(|chunk| chunk.kind == ChunkKind::Function)
                .count(),
            3
        );
    }

    #[test]
    fn detect_chunks_finds_imports() {
        let source = "use std::fmt;\nuse std::path::Path;\n\nfn main() {}\n";
        let chunks = detect_chunks(source, "rs");
        assert_eq!(chunks[0].kind, ChunkKind::Import);
        assert!(chunks[0].content.contains("std::path::Path"));
    }

    #[test]
    fn budget_select_limits_to_seven() {
        let chunks: Vec<_> = (1..=15)
            .map(|line| chunk(ChunkKind::Function, line, line as f64))
            .collect();
        assert_eq!(budget_select(&chunks, None).len(), 7);
    }

    #[test]
    fn budget_select_prefers_tests_over_comments() {
        let mut chunks: Vec<_> = (1..=9)
            .map(|line| chunk(ChunkKind::Comment, line, 100.0))
            .collect();
        chunks.push(chunk(ChunkKind::Test, 10, 1.0));
        assert!(budget_select(&chunks, Some(1)).contains(&9));
    }

    #[test]
    fn render_output_preserves_source_order() {
        let chunks = vec![
            chunk(ChunkKind::Function, 10, 1.0),
            chunk(ChunkKind::Test, 20, 1.0),
        ];
        let output = render_budget_output(&chunks, &[1, 0], "src/lib.rs");
        assert!(
            output.find("line 10").expect("first chunk")
                < output.find("line 20").expect("second chunk")
        );
    }

    #[test]
    fn empty_file_returns_empty_chunks() {
        assert!(detect_chunks(" \n\t", "rs").is_empty());
    }
}
