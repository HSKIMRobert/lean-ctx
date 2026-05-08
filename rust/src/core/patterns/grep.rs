use std::collections::HashMap;

pub fn compress(output: &str) -> Option<String> {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() < 3 {
        return None;
    }

    let mut by_file: HashMap<&str, Vec<(usize, &str)>> = HashMap::new();
    let mut total_matches = 0usize;

    for line in &lines {
        if let Some((file, rest)) = parse_grep_line(line) {
            total_matches += 1;
            let line_num = extract_line_num(rest);
            let content = strip_line_num(rest);
            by_file.entry(file).or_default().push((line_num, content));
        }
    }

    if total_matches == 0 {
        return None;
    }

    let max_matches_per_file = if total_matches > 200 { 5 } else { 10 };

    let mut result = format!("{total_matches} matches in {}F:\n", by_file.len());
    let mut sorted_files: Vec<_> = by_file.iter().collect();
    sorted_files.sort_by_key(|(_, matches)| std::cmp::Reverse(matches.len()));

    for (file, matches) in &sorted_files {
        let short = shorten_path(file);
        result.push_str(&format!("\n{short} ({}):", matches.len()));
        let show = matches.iter().take(max_matches_per_file);
        for (ln, content) in show {
            let trimmed = content.trim();
            let short_content = if trimmed.len() > 120 {
                let truncated: String = trimmed.chars().take(119).collect();
                format!("{truncated}…")
            } else {
                trimmed.to_string()
            };
            if *ln > 0 {
                result.push_str(&format!("\n  {ln}: {short_content}"));
            } else {
                result.push_str(&format!("\n  {short_content}"));
            }
        }
        if matches.len() > max_matches_per_file {
            result.push_str(&format!(
                "\n  ... +{} more",
                matches.len() - max_matches_per_file
            ));
        }
    }

    if result.len() >= output.len() {
        return None;
    }

    Some(result)
}

fn parse_grep_line(line: &str) -> Option<(&str, &str)> {
    if let Some(pos) = line.find(':') {
        let file = &line[..pos];
        if file.contains('/') || file.contains('.') {
            let rest = &line[pos + 1..];
            return Some((file, rest));
        }
    }
    None
}

fn extract_line_num(rest: &str) -> usize {
    if let Some(pos) = rest.find(':') {
        rest[..pos].parse().unwrap_or(0)
    } else {
        0
    }
}

fn strip_line_num(rest: &str) -> &str {
    if let Some(pos) = rest.find(':') {
        if rest[..pos].chars().all(|c| c.is_ascii_digit()) {
            return &rest[pos + 1..];
        }
    }
    rest
}

fn shorten_path(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_grep_output_is_not_claimed_without_matches() {
        assert!(compress("hello\nworld").is_none());
    }

    #[test]
    fn small_grep_output_still_compresses() {
        let output = (0..20)
            .map(|i| format!("src/main.rs:{i}: let x = {i};"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = compress(&output);
        assert!(result.is_some());
        let compressed = result.unwrap();
        assert!(
            compressed.contains("20 matches in 1F:"),
            "should group by file: {compressed}"
        );
        assert!(
            compressed.len() < output.len(),
            "should compress: {} vs {}",
            compressed.len(),
            output.len()
        );
    }

    #[test]
    fn large_output_reduces_per_file_lines() {
        let mut lines = Vec::new();
        for i in 0..250 {
            lines.push(format!("src/a.rs:{i}: line content {i}"));
        }
        let output = lines.join("\n");
        let result = compress(&output).unwrap();
        assert!(
            result.contains("... +245 more"),
            "should show +more for large output: {result}"
        );
    }

    #[test]
    fn non_grep_output_returns_none() {
        let output = "no file:line pattern here\njust regular text\nmore text\nand more";
        assert!(compress(output).is_none());
    }

    #[test]
    fn tiny_grep_output_returns_none_if_inflation() {
        let output = "a.rs:1:x\nb.rs:2:y\nc.rs:3:z\n";
        let result = compress(output);
        if let Some(ref compressed) = result {
            assert!(
                compressed.len() < output.len(),
                "must never inflate: compressed={} vs original={}",
                compressed.len(),
                output.len()
            );
        }
    }

    #[test]
    fn multi_file_many_matches_compresses_well() {
        let mut lines = Vec::new();
        for i in 0..50 {
            lines.push(format!(
                "src/models/user.rs:{}: pub fn method_{i}() {{}}",
                i + 1
            ));
        }
        for i in 0..30 {
            lines.push(format!(
                "src/controllers/auth.rs:{}: let val = method_{i}();",
                i + 1
            ));
        }
        let output = lines.join("\n");
        let result = compress(&output).expect("80 matches should compress");
        assert!(
            result.len() < output.len(),
            "must compress: {} vs {}",
            result.len(),
            output.len()
        );
        assert!(result.contains("80 matches in 2F:"));
        assert!(result.contains("src/models/user.rs (50):"));
        assert!(result.contains("src/controllers/auth.rs (30):"));
    }

    #[test]
    fn many_single_match_files_falls_back_to_none() {
        let lines: Vec<String> = (1..=30)
            .map(|i| format!("src/file{i}.rs:42: fn search_result()"))
            .collect();
        let output = lines.join("\n");
        let result = compress(&output);
        match result {
            Some(ref c) => assert!(
                c.len() < output.len(),
                "if claimed, must be shorter: {} vs {}",
                c.len(),
                output.len()
            ),
            None => {} // correctly declined — overhead too large for 1-match-per-file
        }
    }

    #[test]
    fn never_returns_inflated_output() {
        for count in [3, 5, 10, 15, 25, 50] {
            let lines: Vec<String> = (0..count).map(|i| format!("f{i}.rs:{i}:x")).collect();
            let output = lines.join("\n");
            if let Some(ref c) = compress(&output) {
                assert!(
                    c.len() < output.len(),
                    "count={count}: inflated {} vs {}",
                    c.len(),
                    output.len()
                );
            }
        }
    }
}
