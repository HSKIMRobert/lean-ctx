use std::collections::HashMap;

const MIN_IDENT_LENGTH: usize = 12;
const SHORT_ID_PREFIX: char = 'α';

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SymbolMap {
    forward: HashMap<String, String>,
    next_id: usize,
}

impl SymbolMap {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn register(&mut self, identifier: &str) -> Option<String> {
        if identifier.len() < MIN_IDENT_LENGTH {
            return None;
        }

        if let Some(existing) = self.forward.get(identifier) {
            return Some(existing.clone());
        }

        let short_id = format!("{SHORT_ID_PREFIX}{}", self.next_id);
        self.next_id += 1;
        self.forward
            .insert(identifier.to_string(), short_id.clone());
        Some(short_id)
    }

    pub fn apply(&self, text: &str) -> String {
        if self.forward.is_empty() {
            return text.to_string();
        }

        let mut sorted: Vec<(&String, &String)> = self.forward.iter().collect();
        sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        let mut result = text.to_string();
        for (long, short) in &sorted {
            result = result.replace(long.as_str(), short.as_str());
        }
        result
    }

    pub fn format_table(&self) -> String {
        if self.forward.is_empty() {
            return String::new();
        }

        let mut entries: Vec<(&String, &String)> = self.forward.iter().collect();
        entries.sort_by_key(|(_, v)| {
            v.trim_start_matches(SHORT_ID_PREFIX)
                .parse::<usize>()
                .unwrap_or(0)
        });

        let mut table = String::from("\n§MAP:");
        for (long, short) in &entries {
            table.push_str(&format!("\n  {short}={long}"));
        }
        table
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.forward.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.forward.is_empty()
    }
}

pub fn extract_identifiers(content: &str, ext: &str) -> Vec<String> {
    let ident_re = regex::Regex::new(r"\b[a-zA-Z_][a-zA-Z0-9_]*\b").unwrap();

    let mut seen = HashMap::new();
    for mat in ident_re.find_iter(content) {
        let word = mat.as_str();
        if word.len() >= MIN_IDENT_LENGTH && !is_keyword(word, ext) {
            *seen.entry(word.to_string()).or_insert(0usize) += 1;
        }
    }

    let mut idents: Vec<(String, usize)> = seen.into_iter().collect();
    idents.sort_by(|a, b| {
        let savings_a = a.0.len() * a.1;
        let savings_b = b.0.len() * b.1;
        savings_b.cmp(&savings_a)
    });

    idents.into_iter().map(|(s, _)| s).collect()
}

fn is_keyword(word: &str, ext: &str) -> bool {
    match ext {
        "rs" => matches!(
            word,
            "continue" | "default" | "return" | "struct" | "unsafe" | "where"
        ),
        "ts" | "tsx" | "js" | "jsx" => matches!(
            word,
            "constructor" | "arguments" | "undefined" | "prototype" | "instanceof"
        ),
        "py" => matches!(word, "continue" | "lambda" | "return" | "import" | "class"),
        _ => false,
    }
}
