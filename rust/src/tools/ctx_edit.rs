use std::path::Path;

use crate::core::cache::SessionCache;
use crate::core::tokens::count_tokens;

pub struct EditParams {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
    pub replace_all: bool,
    pub create: bool,
}

pub fn handle(cache: &mut SessionCache, params: EditParams) -> String {
    let file_path = &params.path;

    if params.create {
        return handle_create(cache, file_path, &params.new_string);
    }

    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => return format!("ERROR: cannot read {file_path}: {e}"),
    };

    if params.old_string.is_empty() {
        return "ERROR: old_string must not be empty (use create=true to create a new file)".into();
    }

    let occurrences = content.matches(&params.old_string).count();

    if occurrences == 0 {
        let preview = if params.old_string.len() > 80 {
            format!("{}...", &params.old_string[..77])
        } else {
            params.old_string.clone()
        };
        return format!(
            "ERROR: old_string not found in {file_path}. \
             Make sure it matches exactly (including whitespace/indentation).\n\
             Searched for: {preview}"
        );
    }

    if occurrences > 1 && !params.replace_all {
        return format!(
            "ERROR: old_string found {occurrences} times in {file_path}. \
             Use replace_all=true to replace all, or provide more context to make old_string unique."
        );
    }

    let new_content = if params.replace_all {
        content.replace(&params.old_string, &params.new_string)
    } else {
        content.replacen(&params.old_string, &params.new_string, 1)
    };

    if let Err(e) = std::fs::write(file_path, &new_content) {
        return format!("ERROR: cannot write {file_path}: {e}");
    }

    cache.invalidate(file_path);

    let old_lines = content.lines().count();
    let new_lines = new_content.lines().count();
    let line_delta = new_lines as i64 - old_lines as i64;
    let delta_str = if line_delta > 0 {
        format!("+{line_delta}")
    } else {
        format!("{line_delta}")
    };

    let old_tokens = count_tokens(&params.old_string);
    let new_tokens = count_tokens(&params.new_string);

    let replaced_str = if params.replace_all && occurrences > 1 {
        format!("{occurrences} replacements")
    } else {
        "1 replacement".into()
    };

    let short = Path::new(file_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    format!("✓ {short}: {replaced_str}, {delta_str} lines ({old_tokens}→{new_tokens} tok)")
}

fn handle_create(cache: &mut SessionCache, file_path: &str, content: &str) -> String {
    if let Some(parent) = Path::new(file_path).parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("ERROR: cannot create directory {}: {e}", parent.display());
            }
        }
    }

    if let Err(e) = std::fs::write(file_path, content) {
        return format!("ERROR: cannot write {file_path}: {e}");
    }

    cache.invalidate(file_path);

    let lines = content.lines().count();
    let tokens = count_tokens(content);
    let short = Path::new(file_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    format!("✓ created {short}: {lines} lines, {tokens} tok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn replace_single_occurrence() {
        let f = make_temp("fn hello() {\n    println!(\"hello\");\n}\n");
        let mut cache = SessionCache::new();
        let result = handle(
            &mut cache,
            EditParams {
                path: f.path().to_str().unwrap().to_string(),
                old_string: "hello".into(),
                new_string: "world".into(),
                replace_all: false,
                create: false,
            },
        );
        assert!(result.contains("ERROR"), "should fail: 'hello' appears 2x");
    }

    #[test]
    fn replace_all() {
        let f = make_temp("aaa bbb aaa\n");
        let mut cache = SessionCache::new();
        let result = handle(
            &mut cache,
            EditParams {
                path: f.path().to_str().unwrap().to_string(),
                old_string: "aaa".into(),
                new_string: "ccc".into(),
                replace_all: true,
                create: false,
            },
        );
        assert!(result.contains("2 replacements"));
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(content, "ccc bbb ccc\n");
    }

    #[test]
    fn not_found_error() {
        let f = make_temp("some content\n");
        let mut cache = SessionCache::new();
        let result = handle(
            &mut cache,
            EditParams {
                path: f.path().to_str().unwrap().to_string(),
                old_string: "nonexistent".into(),
                new_string: "x".into(),
                replace_all: false,
                create: false,
            },
        );
        assert!(result.contains("ERROR: old_string not found"));
    }

    #[test]
    fn create_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/new_file.txt");
        let mut cache = SessionCache::new();
        let result = handle(
            &mut cache,
            EditParams {
                path: path.to_str().unwrap().to_string(),
                old_string: String::new(),
                new_string: "line1\nline2\nline3\n".into(),
                replace_all: false,
                create: true,
            },
        );
        assert!(result.contains("created new_file.txt"));
        assert!(result.contains("3 lines"));
        assert!(path.exists());
    }

    #[test]
    fn unique_match_succeeds() {
        let f = make_temp("fn main() {\n    let x = 42;\n}\n");
        let mut cache = SessionCache::new();
        let result = handle(
            &mut cache,
            EditParams {
                path: f.path().to_str().unwrap().to_string(),
                old_string: "let x = 42".into(),
                new_string: "let x = 99".into(),
                replace_all: false,
                create: false,
            },
        );
        assert!(result.contains("✓"));
        assert!(result.contains("1 replacement"));
        let content = std::fs::read_to_string(f.path()).unwrap();
        assert!(content.contains("let x = 99"));
    }
}
