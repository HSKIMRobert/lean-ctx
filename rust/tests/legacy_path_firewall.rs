//! Legacy-path firewall — regression guard for the GH #434–#436 class.
//!
//! Building the HOME-based legacy data dir `~/.lean-ctx` directly (e.g.
//! `home.join(".lean-ctx")`) bypasses the XDG resolver in `core::paths` /
//! `core::data_dir`. That is exactly the "split-brain" those issues fixed: files
//! keep landing in `~/.lean-ctx` even though the resolver has moved everything to
//! the typed `$XDG_*` dirs. New code MUST go through `data_dir()` / `state_dir()`
//! / `config_dir()` / `cache_dir()` instead.
//!
//! This test pins the CURRENT set of files that still construct a home-based
//! legacy path and fails if a NEW one appears, so the debt can only shrink. The
//! allowlist is deliberately exhaustive (not a "src/core only" rule) because the
//! real codebase still carries historical direct writers we want to track.
//!
//! Project-local `<project>/.lean-ctx` directories (the per-repo index, sibling
//! to `.git`) are a different, legitimate concept and are intentionally NOT
//! flagged — only joins rooted at the user HOME are matched.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Files allowed to construct a home-based `~/.lean-ctx` path today.
///
/// Group 1 — resolver / migrator / uninstaller: these MUST know the legacy
/// location by definition. Group 2 — historical direct readers/writers (tracked
/// debt): permitted for now, must not grow; migrate to typed `core::paths`
/// resolvers over time and delete the entry here when done.
const ALLOWLIST: &[&str] = &[
    // Group 1: legitimate owners of the legacy path.
    "core/data_dir.rs",
    "core/paths.rs",
    "core/xdg_migrate.rs",
    "uninstall/agents.rs",
    "uninstall/mod.rs",
    "doctor/common.rs",
    // Group 2: pre-existing direct home-writers/readers (tracked debt).
    "report.rs",
    "cloud_client.rs",
    "cloud_sync.rs",
    "core/slo.rs",
    "core/update_scheduler.rs",
    "core/context_package/keys.rs",
    "core/providers/config_provider/discovery.rs",
    "cli/cloud.rs",
    "cli/wrapped_publish.rs",
    "cli/dispatch/analytics/gain.rs",
    "tui/event_reader.rs",
    "dashboard/routes/agents.rs",
    "tools/ctx_provider.rs",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// A line builds the HOME legacy dir when it joins `".lean-ctx"` and a `home`
/// token appears *before* that join (covers `home.join(".lean-ctx")`,
/// `dirs::home_dir()…join(".lean-ctx")`, `PathBuf::from(home).join(".lean-ctx")`).
/// Project-local joins (`project_root`, `root`, `cwd`, `dir`) have no home token
/// and are ignored by design.
fn builds_home_legacy_path(line: &str) -> bool {
    match line.find(r#"join(".lean-ctx")"#) {
        Some(idx) => line[..idx].contains("home"),
        None => false,
    }
}

#[test]
fn no_new_home_legacy_path_construction() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    assert!(!files.is_empty(), "no source files found under {src:?}");

    let mut offenders = BTreeSet::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        if text.lines().any(builds_home_legacy_path) {
            let rel = file
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            offenders.insert(rel);
        }
    }

    let allow: BTreeSet<String> = ALLOWLIST.iter().map(|s| (*s).to_string()).collect();

    let added: Vec<&String> = offenders.difference(&allow).collect();
    assert!(
        added.is_empty(),
        "New home-based `~/.lean-ctx` path construction detected.\n\
         Use core::paths (data_dir/state_dir/config_dir/cache_dir) instead, or — if\n\
         this is genuinely a resolver/migrator/uninstaller — add the file to\n\
         ALLOWLIST in this test with a reason.\nOffending file(s):\n  {}",
        added
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    let removed: Vec<&String> = allow.difference(&offenders).collect();
    assert!(
        removed.is_empty(),
        "ALLOWLIST has entries that no longer construct a home `~/.lean-ctx` path.\n\
         A writer was migrated to typed dirs — remove it from ALLOWLIST to keep the\n\
         firewall honest:\n  {}",
        removed
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
