//! `lean-ctx cognitive` — display science-driven context intelligence status.

use crate::core::anti_interrupt;
use crate::core::config::CognitiveMode;

/// Run the cognitive status display.
pub(crate) fn run() {
    let config = crate::core::config::Config::load_global();
    let mode = config.cognitive_mode;

    println!("Science-Driven Context Intelligence");
    println!("====================================");
    println!("Mode: {mode}");
    println!();

    match mode {
        CognitiveMode::Off => {
            println!("All science features are disabled.");
            println!("Enable with: lean-ctx config set cognitive_mode basic");
            return;
        }
        CognitiveMode::Basic | CognitiveMode::Full => {}
    }

    let impact = anti_interrupt::compute_impact();
    println!("Anti-Interruption Score: {:.0}%", impact.score * 100.0);
    println!(
        "  Interruptions prevented: {}",
        impact.interruptions_prevented
    );
    println!(
        "  Focus time saved: {:.0} min",
        impact.focus_time_saved_minutes
    );
    println!("  Echo tokens saved: {}", impact.echo_tokens_saved);
    println!();

    println!("Features:");
    println!("  [F1] Information Bottleneck Engine    ✓ active");
    println!("  [F2] Predictive Context Prefetch      ✓ active");
    println!("  [F3] Cognitive Budget Allocator        ✓ active");
    println!("  [F4] Wasserstein Token Allocator       ✓ active");
    println!("  [F5] FSRS Memory Scheduler             ✓ active");
    if mode == CognitiveMode::Full {
        println!("  [F6] On-Demand Graph Expansion        ✓ active");
        println!("  [F7] Anti-Interruption Score           ✓ active");
        println!("  [F8] MDL Read Mode                     ✓ active");
        println!("  [F9] Stigmergic Agent Coordination     ✓ active");
        println!("  [F10] Behavioral Verbosity Learning    ✓ active");
    } else {
        println!("  [F6-F10] Advanced features             - (enable with 'full' mode)");
    }
}
