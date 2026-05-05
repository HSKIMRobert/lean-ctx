use std::io::Read as _;

pub fn cmd_pack(args: &[String]) {
    let project_root = super::common::detect_project_root(args);

    let mut action: Option<&str> = None;
    let mut base: Option<String> = None;
    let mut format: Option<String> = None;
    let mut depth: Option<usize> = None;
    let mut diff_from_stdin = false;

    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        if a == "--pr" {
            action = Some("pr");
            continue;
        }
        if let Some(v) = a.strip_prefix("--base=") {
            base = Some(v.to_string());
            continue;
        }
        if a == "--base" {
            if let Some(v) = it.peek() {
                if !v.starts_with("--") {
                    base = Some((*v).clone());
                    it.next();
                }
            }
            continue;
        }
        if let Some(v) = a.strip_prefix("--format=") {
            format = Some(v.to_string());
            continue;
        }
        if a == "--format" {
            if let Some(v) = it.peek() {
                if !v.starts_with("--") {
                    format = Some((*v).clone());
                    it.next();
                }
            }
            continue;
        }
        if a == "--json" {
            format = Some("json".to_string());
            continue;
        }
        if let Some(v) = a.strip_prefix("--depth=") {
            depth = v.parse::<usize>().ok();
            continue;
        }
        if a == "--depth" {
            if let Some(v) = it.peek() {
                if !v.starts_with("--") {
                    depth = (*v).parse::<usize>().ok();
                    it.next();
                }
            }
            continue;
        }
        if a == "--diff-from-stdin" {
            diff_from_stdin = true;
            continue;
        }
        if !a.starts_with("--") && action.is_none() {
            // Support `lean-ctx pack pr ...`
            if a == "pr" {
                action = Some("pr");
            }
        }
    }

    if action.is_none() {
        action = Some("pr");
    }

    let diff = if diff_from_stdin {
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        if buf.trim().is_empty() {
            None
        } else {
            Some(buf)
        }
    } else {
        None
    };

    match action.unwrap_or("pr") {
        "pr" => {
            let out = crate::tools::ctx_pack::handle(
                "pr",
                &project_root,
                base.as_deref(),
                format.as_deref(),
                depth,
                diff.as_deref(),
            );
            println!("{out}");
        }
        _ => {
            eprintln!(
                "Usage: lean-ctx pack --pr [--base <ref>] [--format markdown|json] [--depth <n>] [--diff-from-stdin] [--root <path>]\n\
                 Examples:\n\
                   lean-ctx pack --pr\n\
                   lean-ctx pack --pr --base main\n\
                   git diff --name-status main...HEAD | lean-ctx pack --pr --diff-from-stdin\n\
                   lean-ctx pack --pr --json\n"
            );
        }
    }
}
