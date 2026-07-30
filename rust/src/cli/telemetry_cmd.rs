//! CLI commands for the anonymous telemetry heartbeat.
//!
//! `lean-ctx telemetry [status|on|off|reset-id|show]`

use crate::core::config;
use crate::core::installation_id;

pub(super) fn cmd_telemetry(args: &[String]) {
    let sub = args.first().map(String::as_str).unwrap_or("status");

    match sub {
        "status" => show_status(),
        "on" | "enable" => set_enabled(true),
        "off" | "disable" => set_enabled(false),
        "reset-id" => reset_id(),
        "show" => show_payload(),
        "history" | "log" => show_history(),
        "--help" | "-h" => print_help(),
        other => {
            eprintln!("telemetry: unknown subcommand '{other}'");
            print_help();
            std::process::exit(1);
        }
    }
}

fn show_status() {
    let cfg = config::Config::load();
    let enabled = cfg.telemetry.enabled;
    let last = cfg.telemetry.last_heartbeat.as_deref().unwrap_or("never");

    println!(
        "  Telemetry:  {}",
        if enabled {
            "\x1b[32menabled\x1b[0m"
        } else {
            "\x1b[2mdisabled\x1b[0m"
        }
    );

    if let Ok(id) = installation_id::get_or_create() {
        println!("  Install ID: {}", installation_id::masked(&id));
    }
    println!("  Last sent:  {last}");
    println!();

    if enabled {
        println!("  \x1b[2mDisable: lean-ctx telemetry off\x1b[0m");
    } else {
        println!("  \x1b[2mEnable:  lean-ctx telemetry on\x1b[0m");
    }
    println!("  \x1b[2mInspect: lean-ctx telemetry show\x1b[0m");
}

fn set_enabled(enabled: bool) {
    match config::setter::set_by_key("telemetry.enabled", if enabled { "true" } else { "false" }) {
        Ok(_) => {
            if enabled {
                println!("Telemetry enabled — thank you for helping improve lean-ctx!");
                println!("Sent daily: version, OS, arch, random install ID. No code, no PII.");
                println!("\x1b[2mDisable anytime: lean-ctx telemetry off\x1b[0m");
            } else {
                println!("Telemetry disabled. No data will be sent.");
                println!("\x1b[2mRe-enable: lean-ctx telemetry on\x1b[0m");
            }
        }
        Err(e) => {
            eprintln!("Failed to update config: {e}");
            std::process::exit(1);
        }
    }
}

fn reset_id() {
    match installation_id::reset() {
        Ok(new_id) => {
            println!(
                "Installation ID regenerated: {}",
                installation_id::masked(&new_id)
            );
            println!("\x1b[2mThe old ID is gone — the server cannot correlate old and new.\x1b[0m");
        }
        Err(e) => {
            eprintln!("Failed to reset installation ID: {e}");
            std::process::exit(1);
        }
    }
}

fn show_payload() {
    let id = installation_id::get_or_create().unwrap_or_else(|_| "<error>".to_string());
    let payload = serde_json::json!({
        "installation_id": id,
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });

    println!("This is the exact JSON that would be sent to api.leanctx.com:");
    println!();
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    );
    println!();
    println!(
        "\x1b[2mEndpoint: POST {}/api/telemetry/heartbeat\x1b[0m",
        api_url()
    );
    println!("\x1b[2mFrequency: at most once per day\x1b[0m");
    println!("\x1b[2mAuthentication: none\x1b[0m");
}

fn api_url() -> String {
    std::env::var("LEAN_CTX_API_URL").unwrap_or_else(|_| "https://api.leanctx.com".to_string())
}

fn show_history() {
    let records = crate::core::telemetry_ledger::read_all();
    if records.is_empty() {
        println!("No heartbeats sent yet.");
        println!("\x1b[2mEnable with: lean-ctx telemetry on\x1b[0m");
        return;
    }
    let header = format!(
        "  \x1b[1m{:<28} {:<12} {:<10} {}\x1b[0m",
        "Timestamp", "Version", "OS", "Arch"
    );
    println!("{header}");
    println!("  {}", "\u{2500}".repeat(65));
    for record in records.iter().rev().take(50) {
        println!(
            "  {:<28} {:<12} {:<10} {}",
            record.timestamp, record.version, record.os, record.arch,
        );
    }
    println!();
    println!(
        "  \x1b[2m{} total heartbeats recorded\x1b[0m",
        records.len()
    );
}

fn print_help() {
    println!("Usage: lean-ctx telemetry [subcommand]");
    println!();
    println!("Manage anonymous usage telemetry (opt-in, no PII).");
    println!();
    println!("Subcommands:");
    println!("  status     Show current telemetry status (default)");
    println!("  on         Enable anonymous heartbeat");
    println!("  off        Disable anonymous heartbeat");
    println!("  show       Display the exact payload that would be sent");
    println!("  reset-id   Regenerate the anonymous installation ID");
    println!("  history    Show log of all sent heartbeats");
    println!();
    println!("The heartbeat sends only: version, OS, architecture, and a random");
    println!("installation UUID. No code, filenames, or personal data — ever.");
}
