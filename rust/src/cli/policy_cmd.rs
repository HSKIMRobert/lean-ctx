//! `lean-ctx policy` — Context Policy Packs v1 (GL #489).
//!
//! Subcommands:
//! * `policy list`             — built-in packs (+ the project pack if present)
//! * `policy show <name>`      — resolved effective policy (`--toml` for raw TOML)
//! * `policy validate [path]`  — lint a pack file (default `.lean-ctx/policy.toml`)

use std::path::{Path, PathBuf};

use crate::core::policy::{self, builtin, PolicyPack, ResolvedPolicy};

/// Project-local pack location, relative to the working directory.
const PROJECT_PACK_PATH: &str = ".lean-ctx/policy.toml";

/// Entry point dispatched from `cli::dispatch`.
pub(crate) fn cmd_policy(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("list") => cmd_list(),
        Some("show") => cmd_show(&args[1..]),
        Some("validate") => cmd_validate(&args[1..]),
        Some("-h" | "--help") | None => print_help(),
        Some(other) => {
            eprintln!("policy: unknown subcommand '{other}'\n");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "lean-ctx policy — context policy packs (governance presets as code)\n\n\
USAGE:\n\
  lean-ctx policy list                 List built-in packs (+ project pack)\n\
  lean-ctx policy show <name> [--toml] Show the resolved effective policy\n\
  lean-ctx policy validate [path]      Validate a pack file\n\
                                       (default: {PROJECT_PACK_PATH})\n\n\
A pack pins governance expectations — default read mode, allowed/denied\n\
tools, redaction patterns, audit retention, context budget — in reviewable\n\
TOML with single inheritance (extends). Start from a built-in:\n\
  lean-ctx policy show baseline --toml > {PROJECT_PACK_PATH}\n\n\
Docs: docs/contracts/context-policy-packs-v1.md · docs/guides/policy-packs.md"
    );
}

// ── list ─────────────────────────────────────────────────────────────────────

fn cmd_list() {
    println!("Built-in policy packs:\n");
    for pack in builtin::all() {
        let extends = pack
            .extends
            .as_deref()
            .map(|p| format!(" (extends {p})"))
            .unwrap_or_default();
        println!("  {:<18} v{}{}", pack.name, pack.version, extends);
        println!("  {:<18} {}", "", pack.description);
    }
    match load_project_pack() {
        Some(Ok(pack)) => {
            println!("\nProject pack ({PROJECT_PACK_PATH}):\n");
            let extends = pack
                .extends
                .as_deref()
                .map(|p| format!(" (extends {p})"))
                .unwrap_or_default();
            println!("  {:<18} v{}{}", pack.name, pack.version, extends);
            println!("  {:<18} {}", "", pack.description);
        }
        Some(Err(e)) => {
            println!("\nProject pack ({PROJECT_PACK_PATH}): INVALID — {e}");
        }
        None => {
            println!("\nNo project pack. Create one from a built-in:");
            println!("  lean-ctx policy show baseline --toml > {PROJECT_PACK_PATH}");
        }
    }
}

/// The project pack, when `.lean-ctx/policy.toml` exists. `None` = no file.
fn load_project_pack() -> Option<Result<PolicyPack, policy::PolicyError>> {
    let path = PathBuf::from(PROJECT_PACK_PATH);
    path.exists().then(|| policy::parse_file(&path))
}

// ── show ─────────────────────────────────────────────────────────────────────

fn cmd_show(args: &[String]) {
    let Some(name) = args.first().filter(|a| !a.starts_with('-')) else {
        eprintln!(
            "policy show: missing pack name (one of: {})",
            builtin::names().join(", ")
        );
        std::process::exit(2);
    };
    let as_toml = args.iter().any(|a| a == "--toml");

    // Resolve "project" / a path-looking argument to the local file, anything
    // else to a built-in.
    let is_toml_path = Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
    let pack = if name == "project" || is_toml_path {
        let path = if name == "project" {
            PathBuf::from(PROJECT_PACK_PATH)
        } else {
            PathBuf::from(name)
        };
        match policy::parse_file(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("policy show: {e}");
                std::process::exit(1);
            }
        }
    } else if let Some(p) = builtin::get(name) {
        p
    } else {
        eprintln!(
            "policy show: no pack named '{name}' (built-ins: {}; or pass a .toml path)",
            builtin::names().join(", ")
        );
        std::process::exit(1);
    };

    if as_toml {
        // Raw, copyable pack definition (not the resolved view) — the natural
        // starting point for an org-specific pack.
        match toml::to_string_pretty(&pack) {
            Ok(t) => print!("{t}"),
            Err(e) => {
                eprintln!("policy show: failed to render TOML: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    match policy::resolve(&pack) {
        Ok(resolved) => print_resolved(&resolved),
        Err(e) => {
            eprintln!("policy show: {e}");
            std::process::exit(1);
        }
    }
}

fn print_resolved(r: &ResolvedPolicy) {
    println!("{} v{} — {}", r.name, r.version, r.description);
    if !r.chain.is_empty() {
        println!("inherits: {}", r.chain.join(" -> "));
    }
    println!();
    println!(
        "  default_read_mode    {}",
        r.default_read_mode.as_deref().unwrap_or("(engine default)")
    );
    match &r.allow_tools {
        Some(allow) => println!("  allow_tools          {}", allow.join(", ")),
        None => println!("  allow_tools          (all tools allowed)"),
    }
    if r.deny_tools.is_empty() {
        println!("  deny_tools           (none)");
    } else {
        println!("  deny_tools           {}", r.deny_tools.join(", "));
    }
    println!(
        "  max_context_tokens   {}",
        r.max_context_tokens
            .map_or("(unbounded)".to_string(), |v| v.to_string())
    );
    println!(
        "  audit_retention_days {}",
        r.audit_retention_days
            .map_or("(unspecified)".to_string(), |v| v.to_string())
    );
    if r.redaction.is_empty() {
        println!("  redaction            (none)");
    } else {
        println!("  redaction            {} patterns:", r.redaction.len());
        for (name, pattern) in &r.redaction {
            println!("    {name:<22} {pattern}");
        }
    }
}

// ── validate ─────────────────────────────────────────────────────────────────

fn cmd_validate(args: &[String]) {
    let path = args
        .first()
        .map_or_else(|| PathBuf::from(PROJECT_PACK_PATH), PathBuf::from);
    if !Path::new(&path).exists() {
        eprintln!("policy validate: {} not found", path.display());
        std::process::exit(1);
    }
    match policy::parse_file(&path).and_then(|p| policy::resolve(&p)) {
        Ok(resolved) => {
            println!(
                "OK — {} v{} validates and resolves ({} redaction patterns, {} denied tools)",
                resolved.name,
                resolved.version,
                resolved.redaction.len(),
                resolved.deny_tools.len()
            );
        }
        Err(e) => {
            eprintln!("INVALID — {e}");
            std::process::exit(1);
        }
    }
}
