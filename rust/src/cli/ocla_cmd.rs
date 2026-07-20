use anyhow::{Result, bail};
use clap::{ArgMatches, Command};

use crate::core::{ocla::OclaService, ocla_bus, savings_ledger};

type TraitEntry = (
    &'static str,
    for<'a> fn(&'a crate::core::ocla::OclaRegistry) -> &'a dyn OclaService,
);

const TRAITS: [TraitEntry; 14] = [
    ("observation_hook", |r| r.observation_hook.as_ref()),
    ("usage_sink", |r| r.usage_sink.as_ref()),
    ("metrics_exporter", |r| r.metrics_exporter.as_ref()),
    ("savings_ledger", |r| r.savings_ledger.as_ref()),
    ("intent_classifier", |r| r.intent_classifier.as_ref()),
    ("outcome_tracker", |r| r.outcome_tracker.as_ref()),
    ("compression_provider", |r| r.compression_provider.as_ref()),
    ("response_optimizer", |r| r.response_optimizer.as_ref()),
    ("model_router", |r| r.model_router.as_ref()),
    ("efficiency_analyzer", |r| r.efficiency_analyzer.as_ref()),
    ("config_tuner", |r| r.config_tuner.as_ref()),
    ("experiment_runner", |r| r.experiment_runner.as_ref()),
    ("connector_scheduler", |r| r.connector_scheduler.as_ref()),
    ("agent_gateway", |r| r.agent_gateway.as_ref()),
];

pub fn register(app: Command) -> Command {
    app.subcommand(
        Command::new("ocla")
            .about("Inspect Open Context & Token Lifecycle Architecture state")
            .subcommand(Command::new("status").about("Show OCLA status and ledger coverage")),
    )
}

pub fn handle(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("ocla", nested)) => return handle(nested),
        Some(("status", _)) | None => print_status(),
        Some((name, _)) => bail!("unknown ocla subcommand: {name}"),
    }
    Ok(())
}

fn print_status() {
    let registry = crate::core::ocla::OclaRegistry::global();
    println!("OCLA traits:");
    for (name, service) in TRAITS {
        let capability = service(registry).capability();
        println!("  {name}: builtin ({:?})", capability.status);
    }

    println!(
        "OclaBus: {} (total events emitted: {})",
        if ocla_bus::is_enabled() {
            "enabled"
        } else {
            "disabled"
        },
        ocla_bus::total_emitted()
    );

    let path = savings_ledger::store::default_path();
    let summary = path
        .as_deref()
        .map(savings_ledger::store::summarize)
        .unwrap_or_default();
    println!(
        "Ledger: total events={}, saved tokens={}, saved USD={:.6}",
        summary.total_events, summary.saved_tokens, summary.saved_usd
    );

    let events = path
        .as_deref()
        .map(savings_ledger::store::load)
        .unwrap_or_default();
    println!("P5 field coverage (with / without):");
    println!(
        "  measurement_method: {} / {}",
        events
            .iter()
            .filter(|e| e.measurement_method.is_some())
            .count(),
        events
            .iter()
            .filter(|e| e.measurement_method.is_none())
            .count()
    );
    println!(
        "  evidence_class: {} / {}",
        events.iter().filter(|e| e.evidence_class.is_some()).count(),
        events.iter().filter(|e| e.evidence_class.is_none()).count()
    );
    println!(
        "  attribution_id: {} / {}",
        events.iter().filter(|e| e.attribution_id.is_some()).count(),
        events.iter().filter(|e| e.attribution_id.is_none()).count()
    );
}

/// Adapter for the existing argument-vector dispatcher.
pub fn cmd_ocla(args: &[String]) {
    let mut argv = vec!["ocla".to_string()];
    argv.extend(args.iter().cloned());
    let matches = register(Command::new("lean-ctx"))
        .try_get_matches_from(argv)
        .unwrap_or_else(|error| error.exit());
    handle(&matches).unwrap_or_else(|error| {
        eprintln!("ocla: {error}");
        std::process::exit(2);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_accepts_status() {
        let matches = register(Command::new("lean-ctx"))
            .try_get_matches_from(["lean-ctx", "ocla", "status"])
            .expect("status should parse");
        let (_, ocla) = matches.subcommand().expect("ocla subcommand");
        assert!(matches!(ocla.subcommand_name(), Some("status")));
    }
}
