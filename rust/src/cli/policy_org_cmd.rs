//! `lean-ctx policy org` — central, signed org policy distribution (GL #674).
//!
//! Admin side: mint an org signing key, sign a pack into a distributable
//! artifact. Endpoint side: pin the org's public key once, then install signed
//! artifacts that the runtime folds in as an un-bypassable floor.
//!
//! Subcommands:
//! * `key --org <name>`              — show/create the org signing key (+ pubkey)
//! * `sign <pack.toml> --org <name>` — build + Ed25519-sign an artifact
//! * `verify <artifact.json>`        — verify signature (+ trust if pinned)
//! * `trust <pubkey> [--org <name>]` — pin a trusted org key (`--list` to show)
//! * `untrust <pubkey>`              — remove a pinned key
//! * `install <artifact.json>`       — verify + trust-check + install for runtime
//! * `uninstall`                     — remove the installed artifact
//! * `status` / `show`               — active org policy + effective floor

use std::path::{Path, PathBuf};

use crate::core::agent_identity;
use crate::core::policy::org::{self, OrgPolicyV1};
use crate::core::policy::{self, ResolvedPolicy, floor};

/// Project-local pack location, relative to the working directory.
const PROJECT_PACK_PATH: &str = ".lean-ctx/policy.toml";

pub(crate) fn cmd_policy_org(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("key") => cmd_key(&args[1..]),
        Some("sign") => cmd_sign(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        Some("trust") => cmd_trust(&args[1..]),
        Some("untrust") => cmd_untrust(&args[1..]),
        Some("install") => cmd_install(&args[1..]),
        Some("uninstall") => cmd_uninstall(),
        Some("status" | "show") => cmd_status(),
        Some("-h" | "--help") | None => print_help(),
        Some(other) => {
            eprintln!("policy org: unknown subcommand '{other}'\n");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "lean-ctx policy org — central, signed org policy distribution\n\n\
USAGE:\n  \
lean-ctx policy org key --org <name>\n      \
Show (create on first use) the org signing key and its public key.\n  \
lean-ctx policy org sign <pack.toml> --org <name> [--policy-version <v>] \\\n      \
[--advisory] [-o <out.json>]\n      \
Build + Ed25519-sign a distributable artifact (default: enforced).\n  \
lean-ctx policy org verify <artifact.json>\n      \
Verify the signature offline (+ trust status if any key is pinned).\n  \
lean-ctx policy org trust <pubkey-hex> [--org <name>] | --list\n      \
Pin a trusted org public key on this endpoint.\n  \
lean-ctx policy org untrust <pubkey-hex>\n  \
lean-ctx policy org install <artifact.json> [--trust]\n      \
Verify + check trust, then install for the runtime to pick up.\n  \
lean-ctx policy org uninstall\n  \
lean-ctx policy org status            Show the active org policy + effective floor\n\n\
A signed org policy is folded in as an un-bypassable FLOOR beneath the local\n\
{PROJECT_PACK_PATH}: the local pack can only tighten it, never weaken it.\n\n\
Docs: docs/contracts/org-policy-v1.md · docs/guides/policy-packs.md"
    );
}

/// Read `--name <value>` from args.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|pos| args.get(pos + 1).cloned())
}

/// First positional (non-`--flag`, not a flag value) argument.
fn first_positional(args: &[String]) -> Option<String> {
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if a.starts_with("--") {
            // Value-bearing flags consume the next token; boolean flags don't.
            if !matches!(a.as_str(), "--advisory" | "--list" | "--trust") {
                skip = true;
            }
            continue;
        }
        if a == "-o" {
            skip = true;
            continue;
        }
        return Some(a.clone());
    }
    None
}

fn require_org(args: &[String], ctx: &str) -> String {
    flag(args, "--org").unwrap_or_else(|| {
        eprintln!("{ctx}: --org <name> is required");
        std::process::exit(2);
    })
}

// ── key ──────────────────────────────────────────────────────────────────────

fn cmd_key(args: &[String]) {
    let org = require_org(args, "policy org key");
    let key_id = org::org_key_id(&org);
    let key = agent_identity::get_or_create_keypair(&key_id).unwrap_or_else(|e| {
        eprintln!("policy org key: {e}");
        std::process::exit(1);
    });
    let pubkey = agent_identity::hex_encode(&key.verifying_key().to_bytes());
    println!("Org signing key for '{org}' (keystore id: {key_id})\n");
    println!("  public key:  {pubkey}\n");
    println!("Distribute this public key to every endpoint and pin it once:");
    println!("  lean-ctx policy org trust {pubkey} --org {org}");
}

// ── sign ─────────────────────────────────────────────────────────────────────

fn cmd_sign(args: &[String]) {
    let org = require_org(args, "policy org sign");
    let Some(pack_path) = first_positional(args) else {
        eprintln!("policy org sign: a pack .toml path is required");
        std::process::exit(2);
    };
    let pack_toml = std::fs::read_to_string(&pack_path).unwrap_or_else(|e| {
        eprintln!("policy org sign: read {pack_path}: {e}");
        std::process::exit(1);
    });
    // Default the distribution version to the pack's own version.
    let policy_version = flag(args, "--policy-version").unwrap_or_else(|| {
        policy::parse(&pack_toml).map_or_else(|_| "1".to_string(), |p| p.version)
    });
    let enforced = !args.iter().any(|a| a == "--advisory");

    let mut artifact = OrgPolicyV1::build(&org, &policy_version, enforced, &pack_toml)
        .unwrap_or_else(|e| {
            eprintln!("policy org sign: {e}");
            std::process::exit(1);
        });
    if let Err(e) = artifact.sign() {
        eprintln!("policy org sign: signing failed: {e}");
        std::process::exit(1);
    }

    let out = flag(args, "-o")
        .or_else(|| flag(args, "--out"))
        .unwrap_or_else(|| "org-policy.signed.json".to_string());
    let json = artifact.to_json().unwrap_or_else(|e| {
        eprintln!("policy org sign: {e}");
        std::process::exit(1);
    });
    if let Err(e) = std::fs::write(&out, json) {
        eprintln!("policy org sign: write {out}: {e}");
        std::process::exit(1);
    }

    let pubkey = artifact.signer_public_key.as_deref().unwrap_or("");
    println!(
        "Signed org policy for '{org}' (version {policy_version}, {}) → {out}",
        if enforced { "enforced" } else { "advisory" }
    );
    println!("  signer public key: {pubkey}");
    println!("\nDistribute {out} + pin the key on each endpoint:");
    println!("  lean-ctx policy org trust {pubkey} --org {org}");
    println!("  lean-ctx policy org install {out}");
}

// ── verify ───────────────────────────────────────────────────────────────────

fn cmd_verify(args: &[String]) {
    let Some(path) = first_positional(args) else {
        eprintln!("policy org verify: an artifact .json path is required");
        std::process::exit(2);
    };
    let artifact = read_artifact(Path::new(&path), "policy org verify");
    let verdict = artifact.verify();
    if !verdict.signature_valid {
        eprintln!(
            "INVALID — {}",
            verdict
                .error
                .as_deref()
                .unwrap_or("signature does not verify")
        );
        std::process::exit(1);
    }
    let pubkey = verdict.signer_public_key.unwrap_or_default();
    println!("VALID — signature verifies (Ed25519, offline)");
    println!("  org:           {}", artifact.org);
    println!("  version:       {}", artifact.policy_version);
    println!(
        "  mode:          {}",
        if artifact.enforced {
            "enforced"
        } else {
            "advisory"
        }
    );
    println!("  signer key:    {pubkey}");
    if org::trust::any_pinned() {
        let trusted = org::trust::is_trusted(&pubkey);
        println!(
            "  trust:         {}",
            if trusted {
                "TRUSTED (key is pinned on this endpoint)"
            } else {
                "NOT TRUSTED (key is not pinned here)"
            }
        );
    } else {
        println!("  trust:         no anchors pinned on this endpoint");
    }
}

// ── trust / untrust ────────────────────────────────────────────────────────────

fn cmd_trust(args: &[String]) {
    if args.iter().any(|a| a == "--list") {
        let keys = org::trust::trusted_keys();
        if keys.is_empty() {
            println!("No org trust anchors pinned on this endpoint.");
            return;
        }
        println!("Pinned org trust anchors:\n");
        for k in keys {
            let src = if k.added_at.is_empty() {
                " (env)".to_string()
            } else {
                String::new()
            };
            println!("  {:<20} {}{}", k.org, k.public_key, src);
        }
        return;
    }
    let Some(pubkey) = first_positional(args) else {
        eprintln!("policy org trust: a public-key hex (or --list) is required");
        std::process::exit(2);
    };
    let org_name = flag(args, "--org").unwrap_or_else(|| "org".to_string());
    match org::trust::pin(&org_name, &pubkey) {
        Ok(_) => println!(
            "Pinned trust anchor for '{org_name}': {}",
            pubkey.trim().to_ascii_lowercase()
        ),
        Err(e) => {
            eprintln!("policy org trust: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_untrust(args: &[String]) {
    let Some(pubkey) = first_positional(args) else {
        eprintln!("policy org untrust: a public-key hex is required");
        std::process::exit(2);
    };
    match org::trust::remove(&pubkey) {
        Ok(true) => println!(
            "Removed trust anchor: {}",
            pubkey.trim().to_ascii_lowercase()
        ),
        Ok(false) => {
            eprintln!("policy org untrust: no such pinned key");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("policy org untrust: {e}");
            std::process::exit(1);
        }
    }
}

// ── install / uninstall ─────────────────────────────────────────────────────────

fn cmd_install(args: &[String]) {
    let Some(path) = first_positional(args) else {
        eprintln!("policy org install: an artifact .json path is required");
        std::process::exit(2);
    };
    let artifact = read_artifact(Path::new(&path), "policy org install");

    let verdict = artifact.verify();
    if !verdict.signature_valid {
        eprintln!(
            "policy org install: refusing to install — signature INVALID ({})",
            verdict.error.as_deref().unwrap_or("does not verify")
        );
        std::process::exit(1);
    }
    let pubkey = verdict.signer_public_key.clone().unwrap_or_default();

    // Trust must be established. `--trust` pins the signer key (TOFU) in one
    // step for first-time setup; otherwise the operator must pin it explicitly.
    if !org::trust::is_trusted(&pubkey) {
        if args.iter().any(|a| a == "--trust") {
            if let Err(e) = org::trust::pin(&artifact.org, &pubkey) {
                eprintln!("policy org install: {e}");
                std::process::exit(1);
            }
            println!("Pinned trust anchor for '{}': {pubkey}", artifact.org);
        } else {
            eprintln!(
                "policy org install: signer key is not trusted on this endpoint.\n  \
                 Pin it first:  lean-ctx policy org trust {pubkey} --org {}\n  \
                 or re-run with --trust to pin it now.",
                artifact.org
            );
            std::process::exit(1);
        }
    }

    let installed = org::store::install(&artifact).unwrap_or_else(|e| {
        eprintln!("policy org install: {e}");
        std::process::exit(1);
    });
    crate::core::policy::runtime::reload();
    println!(
        "Installed org policy '{}' v{} ({}) → {}",
        artifact.org,
        artifact.policy_version,
        if artifact.enforced {
            "enforced"
        } else {
            "advisory (not enforced)"
        },
        installed.display()
    );
    println!();
    cmd_status();
}

fn cmd_uninstall() {
    match org::store::uninstall() {
        Ok(true) => {
            crate::core::policy::runtime::reload();
            println!("Removed the installed org policy.");
        }
        Ok(false) => println!("No org policy is installed."),
        Err(e) => {
            eprintln!("policy org uninstall: {e}");
            std::process::exit(1);
        }
    }
}

// ── status / show ──────────────────────────────────────────────────────────────

fn cmd_status() {
    let s = org::status();
    if !s.present {
        println!("Org policy: none installed.");
        println!("  pinned trust anchors: {}", s.pinned_anchors);
        if s.pinned_anchors == 0 {
            println!("\nThis endpoint has no central policy. Pin a key and install one:");
            println!("  lean-ctx policy org trust <pubkey> --org <name>");
            println!("  lean-ctx policy org install <artifact.json>");
        }
        return;
    }

    println!(
        "Org policy: {}",
        if s.applied {
            "ENFORCED"
        } else {
            "present, NOT enforced"
        }
    );
    println!("  org:           {}", s.org.as_deref().unwrap_or("-"));
    println!(
        "  version:       {}",
        s.policy_version.as_deref().unwrap_or("-")
    );
    println!("  issued at:     {}", s.issued_at.as_deref().unwrap_or("-"));
    println!(
        "  source:        {}",
        s.source
            .as_ref()
            .map_or("-".into(), |p| p.display().to_string())
    );
    println!(
        "  signature:     {}",
        if s.signature_valid {
            "valid (Ed25519)"
        } else {
            "INVALID"
        }
    );
    println!(
        "  signer key:    {}",
        s.signer_public_key.as_deref().unwrap_or("-")
    );
    println!(
        "  trust:         {}",
        if s.trusted {
            "trusted (pinned)"
        } else {
            "NOT trusted (pin the signer key)"
        }
    );
    println!(
        "  mode:          {}",
        if s.enforced { "enforced" } else { "advisory" }
    );
    if let Some(err) = &s.resolve_error {
        println!("  resolve error: {err}");
    }
    println!("  pinned anchors: {}", s.pinned_anchors);

    // Show the effective floor merge for inspection, even when not yet applied,
    // so an admin can preview exactly what the endpoint would enforce.
    if let Some(effective) = effective_preview() {
        println!("\nEffective policy (org floor ⊕ local pack):\n");
        print_resolved(&effective);
    }
}

/// Build the would-be effective policy for `show`: the artifact's pack as the
/// floor merged with the local project pack — independent of trust/enforced, so
/// it is a true preview of what enforcement would look like once turned on.
fn effective_preview() -> Option<ResolvedPolicy> {
    let artifact = org::store::load_active()?;
    let org_resolved = artifact.resolved().ok()?;
    Some(floor::merge_floor(&org_resolved, local_pack().as_ref()))
}

/// The resolved local project pack, if present and valid.
fn local_pack() -> Option<ResolvedPolicy> {
    let path = PathBuf::from(PROJECT_PACK_PATH);
    if !path.exists() {
        return None;
    }
    policy::parse_file(&path)
        .and_then(|p| policy::resolve(&p))
        .ok()
}

fn read_artifact(path: &Path, ctx: &str) -> OrgPolicyV1 {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{ctx}: read {}: {e}", path.display());
        std::process::exit(1);
    });
    OrgPolicyV1::from_json(&text).unwrap_or_else(|e| {
        eprintln!("{ctx}: {e}");
        std::process::exit(1);
    })
}

/// Compact resolved-policy renderer (mirrors `policy show`).
fn print_resolved(r: &ResolvedPolicy) {
    println!("  {} v{} — {}", r.name, r.version, r.description);
    if !r.chain.is_empty() {
        println!("  inherits: {}", r.chain.join(" -> "));
    }
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
    if !r.redaction.is_empty() {
        println!("  redaction            {} patterns", r.redaction.len());
    }
    if !r.filters.is_empty() {
        let f = &r.filters;
        println!(
            "  filters              pii={} classification={} injection={}",
            f.pii.as_deref().unwrap_or("off"),
            f.classification.as_deref().unwrap_or("off"),
            f.injection.as_deref().unwrap_or("off"),
        );
    }
    if !r.egress.is_empty() {
        println!(
            "  egress               {} forbidden patterns, block_secrets={}",
            r.egress.forbidden_patterns.len(),
            r.egress.block_secrets.unwrap_or(false),
        );
    }
}
