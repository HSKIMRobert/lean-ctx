//! `lean-ctx team …` — OSS stub (ADR-023).
//!
//! Team server management commands (serve, token, sync, slo-report) require
//! the enterprise edition. This stub prints a notice and exits.

pub(crate) fn cmd_team(_rest: &[String]) {
    eprintln!("lean-ctx team: requires lean-ctx Enterprise edition.");
    eprintln!("See https://leanctx.dev/enterprise for details.");
    std::process::exit(1);
}
