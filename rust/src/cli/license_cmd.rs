//! `lean-ctx license` CLI (GL #667) — the self-hosted Enterprise license.
//!
//! Two sides share one command surface:
//! - **Vendor** mints licenses: `keygen` (bootstrap a signing key), `issue`
//!   (build + Ed25519-sign a license for a customer).
//! - **Customer** installs them on an air-gapped self-host: `install`, `status`,
//!   `verify`, `uninstall` — all offline, no control-plane round-trip.
//!
//! The grant only ever *unlocks commercial/hosted* entitlements; it never gates
//! a local capability (Local-Free Invariant).

use std::path::Path;

use ed25519_dalek::SigningKey;

use crate::core::agent_identity::{hex_decode, hex_encode};
use crate::core::billing::Plan;
use crate::core::license::{self, model::LicenseV1, store};

const SIGNING_KEY_ENV: &str = "LEAN_CTX_LICENSE_SIGNING_KEY";

pub(crate) fn cmd_license(args: &[String]) {
    match args.first().map(String::as_str) {
        Some("keygen") => cmd_keygen(&args[1..]),
        Some("issue") => cmd_issue(&args[1..]),
        Some("install") => cmd_install(&args[1..]),
        Some("status" | "show") => cmd_status(&args[1..]),
        Some("verify") => cmd_verify(&args[1..]),
        Some("uninstall" | "remove") => cmd_uninstall(),
        Some("-h" | "--help") | None => print_help(),
        Some(other) => {
            eprintln!("license: unknown subcommand '{other}'\n");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "lean-ctx license — self-hosted Enterprise license (offline entitlement validation)\n\n\
USAGE:\n  \
lean-ctx license status [--json]            show the installed license + effective grant\n  \
lean-ctx license verify <file> [--json]     offline-verify a license artifact\n  \
lean-ctx license install <file>             verify, then install for this machine\n  \
lean-ctx license uninstall                  remove the installed license\n\n\
VENDOR (mint licenses — needs the signing key):\n  \
lean-ctx license keygen [--out <key>]       create a vendor signing keypair\n  \
lean-ctx license issue --customer <id> [--plan enterprise] \\\n      \
[--expires <rfc3339> | --days <N>] [--audit-retention-days <N>] \\\n      \
[--note <text>] [--key <path|hex>] [--out <file>]\n\n\
The signing key is read from --key or $LEAN_CTX_LICENSE_SIGNING_KEY (a 32-byte\n\
key file or a 64-char hex string). Customers trust the matching public key via\n\
the compiled-in vendor anchor or $LEAN_CTX_LICENSE_PUBKEY.\n\n\
EXAMPLES:\n  \
lean-ctx license keygen --out vendor-signing.key\n  \
lean-ctx license issue --customer LGT-Bank --days 365 --note MoU-2026 --out lgt.json\n  \
lean-ctx license install lgt.json && lean-ctx license status"
    );
}

/// Flag value lookup (`--name value`), mirroring the other CLI modules.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|pos| args.get(pos + 1).cloned())
}

fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

// ---------------------------------------------------------------------------
// vendor side
// ---------------------------------------------------------------------------

fn cmd_keygen(args: &[String]) {
    let mut seed = [0u8; 32];
    if let Err(e) = getrandom::fill(&mut seed) {
        eprintln!("license keygen: CSPRNG unavailable: {e}");
        std::process::exit(1);
    }
    let key = SigningKey::from_bytes(&seed);
    let pubkey = hex_encode(&key.verifying_key().to_bytes());
    let out = flag(args, "--out").unwrap_or_else(|| "vendor-signing.key".to_string());

    if let Err(e) = write_secret(Path::new(&out), &key.to_bytes()) {
        eprintln!("license keygen: {e}");
        std::process::exit(1);
    }

    println!("Vendor signing keypair created.\n");
    println!("  Private key (KEEP SECRET): {out}");
    println!("  Public key  (trust anchor): {pubkey}\n");
    println!("Distribute trust to customers via either:");
    println!("  - compile-in: add the public key to VENDOR_PUBLIC_KEYS in core/license/mod.rs");
    println!("  - runtime:    export LEAN_CTX_LICENSE_PUBKEY={pubkey}\n");
    println!("Issue a license:");
    println!("  lean-ctx license issue --customer <id> --days 365 --key {out} --out license.json");
}

fn cmd_issue(args: &[String]) {
    let Some(customer) = flag(args, "--customer") else {
        eprintln!("license issue: --customer <id> is required\n");
        print_help();
        std::process::exit(2);
    };

    let plan = Plan::parse(&flag(args, "--plan").unwrap_or_else(|| "enterprise".to_string()));

    let expires_at = match resolve_expiry(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("license issue: {e}");
            std::process::exit(2);
        }
    };

    let audit_retention_days = match flag(args, "--audit-retention-days") {
        None => None,
        Some(s) => {
            let Ok(n) = s.parse::<u32>() else {
                eprintln!("license issue: --audit-retention-days must be a number");
                std::process::exit(2);
            };
            Some(n)
        }
    };

    let note = flag(args, "--note");

    let key = match resolve_signing_key(flag(args, "--key")) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("license issue: {e}");
            std::process::exit(1);
        }
    };

    let mut lic = LicenseV1::new(&customer, plan, expires_at, audit_retention_days, note);
    lic.sign_with_key(&key);

    let signer = lic.signer_public_key.clone().unwrap_or_default();
    let out = flag(args, "--out").unwrap_or_else(|| "license.json".to_string());
    let json = match lic.to_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("license issue: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = std::fs::write(&out, json) {
        eprintln!("license issue: write {out}: {e}");
        std::process::exit(1);
    }

    println!("Signed license written to {out}");
    println!("  Customer:   {customer}");
    println!("  Plan:       {}", plan.as_str());
    println!("  Signer key: {signer}");
    if !license::trusted_keys()
        .iter()
        .any(|k| k.eq_ignore_ascii_case(&signer))
    {
        println!(
            "\n  ! This signer is NOT a trusted anchor on this machine. Customers must trust it\n    \
(compile it into VENDOR_PUBLIC_KEYS or set LEAN_CTX_LICENSE_PUBKEY={signer})."
        );
    }
}

// ---------------------------------------------------------------------------
// customer side
// ---------------------------------------------------------------------------

fn cmd_install(args: &[String]) {
    let Some(path) = args.first().filter(|a| !a.starts_with('-')) else {
        eprintln!("license install: a license file path is required\n");
        print_help();
        std::process::exit(2);
    };
    let lic = match store::read(Path::new(path)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("license install: {e}");
            std::process::exit(1);
        }
    };
    let res = lic.verify_against(&license::trusted_keys());
    if !res.ok() {
        eprintln!(
            "license install: refused — {}",
            res.error.as_deref().unwrap_or("not trusted")
        );
        std::process::exit(1);
    }
    let now = chrono::Utc::now();
    if lic.is_expired(now) && lic.days_past_expiry(now) > license::LICENSE_GRACE_DAYS {
        eprintln!(
            "license install: refused — expired {}d ago (beyond {}d grace)",
            lic.days_past_expiry(now),
            license::LICENSE_GRACE_DAYS
        );
        std::process::exit(1);
    }
    let installed = match store::install(&lic) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("license install: {e}");
            std::process::exit(1);
        }
    };
    println!("License installed to {}", installed.display());
    print_status_text(&license::status());
}

fn cmd_status(args: &[String]) {
    let st = license::status();
    if has(args, "--json") {
        print_json(&st);
        return;
    }
    print_status_text(&st);
}

fn cmd_verify(args: &[String]) {
    let Some(path) = args.first().filter(|a| !a.starts_with('-')) else {
        eprintln!("license verify: a license file path is required\n");
        print_help();
        std::process::exit(2);
    };
    let lic = match store::read(Path::new(path)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("license verify: {e}");
            std::process::exit(1);
        }
    };
    let res = lic.verify_against(&license::trusted_keys());
    let now = chrono::Utc::now();
    let expired = lic.is_expired(now);
    let within_grace = expired && lic.days_past_expiry(now) <= license::LICENSE_GRACE_DAYS;
    let usable = res.ok() && (!expired || within_grace);

    if has(args, "--json") {
        let payload = serde_json::json!({
            "signature_valid": res.signature_valid,
            "trusted": res.trusted,
            "signer_public_key": res.signer_public_key,
            "customer": lic.customer,
            "plan": lic.plan.as_str(),
            "expires_at": lic.expires_at,
            "expired": expired,
            "within_grace": within_grace,
            "usable": usable,
            "error": res.error,
        });
        print_json(&payload);
        return;
    }

    if usable {
        println!("VALID — license verifies (Ed25519, offline) and is trusted");
    } else if res.signature_valid && !res.trusted {
        println!("UNTRUSTED — signature is valid but the signer is not a trusted vendor key");
    } else if expired && !within_grace {
        println!("EXPIRED — signature valid but past the grace window");
    } else {
        eprintln!(
            "INVALID — {}",
            res.error.as_deref().unwrap_or("signature does not verify")
        );
        std::process::exit(1);
    }
    println!("  Customer:   {}", lic.customer);
    println!("  Plan:       {}", lic.plan.as_str());
    if let Some(pk) = &res.signer_public_key {
        println!("  Signer key: {pk}");
    }
    if let Some(exp) = &lic.expires_at {
        println!("  Expires:    {exp}");
    } else {
        println!("  Expires:    never");
    }
    if !usable {
        std::process::exit(1);
    }
}

fn cmd_uninstall() {
    match store::uninstall() {
        Ok(true) => println!("License removed."),
        Ok(false) => println!("No installed license."),
        Err(e) => {
            eprintln!("license uninstall: {e}");
            std::process::exit(1);
        }
    }
}

fn print_status_text(st: &license::LicenseStatus) {
    println!("lean-ctx license status\n");
    if !st.installed {
        println!("  No license installed (self-host runs on the free Local plane).");
        println!("  Install one:  lean-ctx license install <file>");
        return;
    }
    if let Some(p) = &st.source_path {
        println!("  Source:     {p}");
    }
    if let Some(err) = &st.error
        && st.plan.is_none()
    {
        println!("  ! Unreadable: {err}");
        return;
    }
    println!("  Customer:   {}", st.customer.as_deref().unwrap_or("-"));
    println!("  Plan:       {}", st.plan.as_deref().unwrap_or("-"));
    println!("  Issued:     {}", st.issued_at.as_deref().unwrap_or("-"));
    println!(
        "  Expires:    {}",
        st.expires_at.as_deref().unwrap_or("never")
    );
    println!(
        "  Signature:  {}",
        if st.signature_valid {
            "valid"
        } else {
            "INVALID"
        }
    );
    println!(
        "  Trusted:    {}",
        if st.trusted {
            "yes"
        } else {
            "NO (signer is not a trusted vendor key)"
        }
    );
    if st.expired {
        if st.within_grace {
            println!(
                "  Expiry:     expired {}d ago — within {}d grace",
                st.days_past_expiry,
                license::LICENSE_GRACE_DAYS
            );
        } else {
            println!(
                "  Expiry:     EXPIRED {}d ago (beyond grace)",
                st.days_past_expiry
            );
        }
    }
    if let Some(days) = st.audit_retention_days {
        println!("  Audit:      {days} days (license override)");
    }
    if let Some(note) = &st.note {
        println!("  Note:       {note}");
    }
    println!();
    if st.active {
        let plan = Plan::parse(st.plan.as_deref().unwrap_or("free"));
        let e = plan.entitlements();
        println!(
            "  ACTIVE — granting {} entitlements offline:",
            plan.as_str()
        );
        println!(
            "    sso_oidc: {}   sso_scim: {}   audit_retention_days: {}",
            e.sso_oidc, e.sso_scim, e.audit_retention_days
        );
    } else {
        println!("  NOT ACTIVE — not currently granting entitlements.");
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Resolve `--expires <rfc3339>` or `--days <N>` into an optional expiry. No flag
/// → perpetual license. The two are mutually exclusive.
fn resolve_expiry(args: &[String]) -> Result<Option<String>, String> {
    match (flag(args, "--expires"), flag(args, "--days")) {
        (Some(_), Some(_)) => Err("use either --expires or --days, not both".to_string()),
        (Some(ts), None) => {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .map_err(|_| format!("--expires must be RFC 3339 (got '{ts}')"))?;
            Ok(Some(ts))
        }
        (None, Some(days)) => {
            let n: i64 = days
                .parse()
                .map_err(|_| "--days must be a whole number".to_string())?;
            let exp = chrono::Utc::now() + chrono::Duration::days(n);
            Ok(Some(exp.to_rfc3339()))
        }
        (None, None) => Ok(None),
    }
}

/// Load the vendor signing key from `--key` (path or hex) or the
/// `LEAN_CTX_LICENSE_SIGNING_KEY` env (path or hex).
fn resolve_signing_key(arg: Option<String>) -> Result<SigningKey, String> {
    let src = arg
        .or_else(|| std::env::var(SIGNING_KEY_ENV).ok())
        .ok_or_else(|| {
            format!(
                "no signing key — pass --key <path|hex> or set {SIGNING_KEY_ENV}.\n  \
Bootstrap one with: lean-ctx license keygen"
            )
        })?;
    parse_signing_key(&src)
}

/// Accept a 32-byte raw key file, a file holding 64 hex chars, or a bare 64-char
/// hex string. Returns the Ed25519 signing key.
fn parse_signing_key(src: &str) -> Result<SigningKey, String> {
    let path = Path::new(src);
    if path.exists() {
        let bytes = std::fs::read(path).map_err(|e| format!("read key {src}: {e}"))?;
        if bytes.len() == 32 {
            let arr: [u8; 32] = bytes.try_into().expect("checked len 32");
            return Ok(SigningKey::from_bytes(&arr));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| "key file is neither 32 bytes nor hex".to_string())?;
        return key_from_hex(text.trim());
    }
    key_from_hex(src.trim())
}

fn key_from_hex(hex: &str) -> Result<SigningKey, String> {
    let bytes = hex_decode(hex).map_err(|e| format!("invalid key hex: {e}"))?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "signing key must be 32 bytes (64 hex chars)".to_string())?;
    Ok(SigningKey::from_bytes(&arr))
}

/// Pretty-print a serializable value as JSON, or exit non-zero on failure.
fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => println!("{s}"),
        Err(e) => {
            eprintln!("license: JSON serialization failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Write a secret key file with `0600` perms on Unix.
fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
