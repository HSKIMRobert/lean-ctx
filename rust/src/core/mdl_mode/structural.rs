//! Minimum description length (MDL) structural fingerprints for source files.
//!
//! Replaces full source with compact type and function signatures to minimize
//! token cost while preserving navigable structure.

use std::fmt::Write as _;

use crate::core::signatures::{Signature, extract_signatures};
use crate::core::tokens::count_tokens;

/// A structural description of a source file.
#[derive(Debug, Clone)]
pub struct StructuralDescription {
    /// File path.
    pub path: String,
    /// Module-level doc comment (first line only).
    pub module_doc: Option<String>,
    /// Exported types (structs, enums, traits).
    pub types: Vec<TypeFingerprint>,
    /// Exported functions and methods.
    pub functions: Vec<FunctionFingerprint>,
    /// Number of import declarations.
    pub import_count: usize,
    /// Total lines in original file.
    pub total_lines: usize,
    /// Token count of this structural description.
    pub description_tokens: usize,
    /// Token count of the original file.
    pub original_tokens: usize,
}

/// Compact identity and member count for an exported type.
#[derive(Debug, Clone)]
pub struct TypeFingerprint {
    /// Declared type name.
    pub name: String,
    /// Type category such as `struct`, `enum`, or `trait`.
    pub kind: &'static str,
    /// Number of fields, variants, or trait members.
    pub field_count: usize,
    /// Whether the declaration is exported.
    pub is_exported: bool,
}

/// Compact signature and complexity classification for an exported function.
#[derive(Debug, Clone)]
pub struct FunctionFingerprint {
    /// Declared function or method name.
    pub name: String,
    /// Compact parameter text.
    pub params: String,
    /// Declared return type.
    pub return_type: String,
    /// Whether the function is asynchronous.
    pub is_async: bool,
    /// Whether the function is exported.
    pub is_exported: bool,
    /// Coarse size classification: `simple`, `moderate`, or `complex`.
    pub complexity_hint: &'static str,
}

/// Generate a structural description from source code.
pub(crate) fn generate_structural_description(
    content: &str,
    path: &str,
    file_ext: &str,
) -> StructuralDescription {
    let lines: Vec<&str> = content.lines().collect();
    let signatures = extract_signatures(content, file_ext.trim_start_matches('.'));

    let types = signatures
        .iter()
        .filter(|signature| {
            signature.is_exported && matches!(signature.kind, "struct" | "enum" | "trait")
        })
        .map(|signature| TypeFingerprint {
            name: signature.name.clone(),
            kind: signature.kind,
            field_count: type_member_count(signature, &lines),
            is_exported: signature.is_exported,
        })
        .collect();

    let functions = signatures
        .iter()
        .filter(|signature| signature.is_exported && matches!(signature.kind, "fn" | "method"))
        .map(function_fingerprint)
        .collect();

    let original_tokens = count_tokens(content);
    let mut description = StructuralDescription {
        path: path.to_string(),
        module_doc: extract_module_doc(content),
        types,
        functions,
        import_count: count_imports(content),
        total_lines: lines.len(),
        description_tokens: 0,
        original_tokens,
    };

    if !content.is_empty() {
        description.description_tokens = count_tokens(&description.render());
    }
    description
}

impl StructuralDescription {
    /// Render as compact text for LLM context.
    pub fn render(&self) -> String {
        let mut rendered = format!(
            "# {} ({} lines, {} tokens → {} tokens structural)",
            self.path, self.total_lines, self.original_tokens, self.description_tokens
        );

        if let Some(module_doc) = &self.module_doc {
            let _ = write!(rendered, "\n## Module doc: {module_doc}");
        }
        if !self.types.is_empty() {
            rendered.push_str("\n## Types: ");
            let type_text = self
                .types
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ");
            rendered.push_str(&type_text);
        }
        if !self.functions.is_empty() {
            rendered.push_str("\n## Functions: ");
            let function_text = self
                .functions
                .iter()
                .map(render_function)
                .collect::<Vec<_>>()
                .join(", ");
            rendered.push_str(&function_text);
        }
        let _ = write!(rendered, "\n## Imports: {} imports", self.import_count);
        rendered
    }

    /// Return structural token count divided by original token count.
    pub fn compression_ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            return 1.0;
        }
        self.description_tokens as f64 / self.original_tokens as f64
    }
}

fn function_fingerprint(signature: &Signature) -> FunctionFingerprint {
    let line_count = signature
        .start_line
        .zip(signature.end_line)
        .map_or(1, |(start, end)| end.saturating_sub(start) + 1);
    let param_count = signature
        .params
        .split(',')
        .filter(|param| !param.trim().is_empty())
        .count();
    let complexity_hint = if line_count > 30 || param_count > 5 || signature.is_async {
        "complex"
    } else if line_count > 10 || param_count >= 3 {
        "moderate"
    } else {
        "simple"
    };

    FunctionFingerprint {
        name: signature.name.clone(),
        params: signature.params.clone(),
        return_type: signature.return_type.clone(),
        is_async: signature.is_async,
        is_exported: signature.is_exported,
        complexity_hint,
    }
}

fn type_member_count(signature: &Signature, lines: &[&str]) -> usize {
    let Some(start) = signature.start_line else {
        return 0;
    };
    let end = signature.end_line.unwrap_or(start).min(lines.len());
    let Some(source) = lines.get(start.saturating_sub(1)..end) else {
        return 0;
    };
    let declaration = source.join("\n");
    count_braced_members(&declaration)
}

fn count_braced_members(declaration: &str) -> usize {
    let Some(open) = declaration.find('{') else {
        return 0;
    };
    let mut depth = 1_usize;
    let mut count = 0_usize;
    let mut has_member_text = false;

    for character in declaration[open + 1..].chars() {
        match character {
            '{' => {
                if depth == 1 {
                    has_member_text = true;
                }
                depth += 1;
            }
            '}' => {
                if depth == 1 {
                    break;
                }
                depth -= 1;
            }
            ',' | ';' if depth == 1 => {
                if has_member_text {
                    count += 1;
                    has_member_text = false;
                }
            }
            character if depth == 1 && !character.is_whitespace() => has_member_text = true,
            _ => {}
        }
    }
    count + usize::from(has_member_text)
}

fn extract_module_doc(content: &str) -> Option<String> {
    let mut in_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(doc) = trimmed.strip_prefix("//!") {
            return nonempty_doc(doc);
        }
        if let Some(rest) = trimmed.strip_prefix("/**") {
            in_block = true;
            let first_line = rest.split("*/").next().unwrap_or_default();
            if let Some(doc) = nonempty_doc(first_line) {
                return Some(doc);
            }
        } else if in_block {
            let doc_line = trimmed.trim_start_matches('*');
            if let Some(doc) = nonempty_doc(doc_line.split("*/").next().unwrap_or_default()) {
                return Some(doc);
            }
            if trimmed.contains("*/") {
                in_block = false;
            }
        }
    }
    None
}

fn nonempty_doc(text: &str) -> Option<String> {
    let doc = text.trim().trim_end_matches("*/").trim();
    (!doc.is_empty()).then(|| doc.to_string())
}

fn count_imports(content: &str) -> usize {
    content
        .lines()
        .map(str::trim_start)
        .filter(|line| {
            line.starts_with("use ")
                || line.starts_with("pub use ")
                || line.starts_with("import ")
                || line.starts_with("from ")
                || line.starts_with("#include")
        })
        .count()
}

fn render_type(fingerprint: &TypeFingerprint) -> String {
    let label = match fingerprint.kind {
        "struct" => "Struct",
        "enum" => "Enum",
        "trait" => "Trait",
        _ => "Type",
    };
    let member_label = if fingerprint.kind == "enum" {
        "variants"
    } else {
        "fields"
    };
    format!(
        "{label} {}({} {member_label})",
        fingerprint.name, fingerprint.field_count
    )
}

fn render_function(fingerprint: &FunctionFingerprint) -> String {
    let return_type = if fingerprint.return_type.is_empty() {
        String::new()
    } else {
        format!(" -> {}", fingerprint.return_type)
    };
    let async_marker = if fingerprint.is_async { " [async]" } else { "" };
    let export_marker = if fingerprint.is_exported {
        " [pub]"
    } else {
        ""
    };
    format!(
        "{}({}){return_type}{async_marker}{export_marker} [{}]",
        fingerprint.name, fingerprint.params, fingerprint.complexity_hint
    )
}

#[cfg(test)]
mod tests {
    use super::generate_structural_description;

    const RUST_SOURCE: &str = r"//! User model and lookup helpers.
use std::collections::HashMap;

pub struct User {
    pub id: u64,
    pub name: String,
}

pub fn find_user(users: &HashMap<u64, User>, id: u64) -> Option<&User> {
    users.get(&id)
}
";

    #[test]
    fn generate_for_rust_file_with_struct_and_fn() {
        let desc = generate_structural_description(RUST_SOURCE, "src/user.rs", "rs");

        assert_eq!(desc.types.len(), 1);
        assert_eq!(desc.types[0].name, "User");
        assert_eq!(desc.types[0].field_count, 2);
        assert_eq!(desc.functions.len(), 1);
        assert_eq!(desc.functions[0].name, "find_user");
        assert_eq!(desc.import_count, 1);
    }

    #[test]
    fn render_includes_types_and_functions() {
        let rendered = generate_structural_description(RUST_SOURCE, "src/user.rs", "rs").render();

        assert!(rendered.contains("Struct User(2 fields)"));
        assert!(
            rendered.contains("find_user("),
            "render should include find_user function"
        );
    }

    #[test]
    fn compression_ratio_is_less_than_one() {
        let body = "    let value = id + 1;\n".repeat(80);
        let source = format!("pub fn transform(id: u64) -> u64 {{\n{body}    id\n}}\n");
        let desc = generate_structural_description(&source, "src/large.rs", "rs");

        assert!(desc.compression_ratio() < 1.0);
    }

    #[test]
    fn empty_file_produces_minimal_description() {
        let desc = generate_structural_description("", "empty.rs", "rs");

        assert!(desc.types.is_empty());
        assert!(desc.functions.is_empty());
        assert_eq!(desc.total_lines, 0);
        assert_eq!(desc.description_tokens, 0);
    }

    #[test]
    fn module_doc_extracted_correctly() {
        let desc = generate_structural_description(RUST_SOURCE, "src/user.rs", "rs");

        assert_eq!(
            desc.module_doc.as_deref(),
            Some("User model and lookup helpers.")
        );
    }
}
