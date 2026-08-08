//! Cognitive-load impact metrics derived from anti-interruption events.
//!
//! Estimates focus time saved and extraneous load reduction using Gloria Mark's
//! 23-minute context-switch recovery model.

use super::tracker::{prevented_counts, session_interruptions};

/// Cognitive impact report for the current session.
#[derive(Debug, Clone)]
pub(crate) struct CognitiveImpactReport {
    /// Total interruption events prevented.
    pub interruptions_prevented: u64,
    /// Context switches prevented (each saves ~23 min recovery time).
    pub context_switches_saved: u64,
    /// Echo tokens that were NOT re-sent.
    pub echo_tokens_saved: u64,
    /// Estimated cognitive load reduction (0.0-1.0).
    pub cognitive_load_reduction: f64,
    /// Estimated focus time saved in minutes (based on 23min per context switch).
    pub focus_time_saved_minutes: f64,
    /// Anti-interruption score (0.0 = terrible, 1.0 = perfect).
    pub score: f64,
}

/// Compute cognitive impact report from session events.
pub(crate) fn compute_impact() -> CognitiveImpactReport {
    let counts = prevented_counts();
    let events = session_interruptions();
    let total_events = events.len() as u64;
    let total_prevented = events.iter().filter(|(_, prevented)| *prevented).count() as u64;

    // Score = prevented / (prevented + occurred).
    // If no events exist, nothing interrupted the session, so the score is perfect.
    let score = if total_events == 0 {
        1.0
    } else {
        total_prevented as f64 / total_events as f64
    };

    // Context switch recovery time: 23 minutes per switch (Gloria Mark, 2023).
    let focus_minutes = counts.context_switches_prevented as f64 * 23.0;

    // Each prevented interruption reduces an estimated 2% of extraneous load.
    let load_reduction = (total_prevented as f64 * 0.02).min(1.0);

    CognitiveImpactReport {
        interruptions_prevented: total_prevented,
        context_switches_saved: counts.context_switches_prevented,
        echo_tokens_saved: counts.echo_prevented,
        cognitive_load_reduction: load_reduction,
        focus_time_saved_minutes: focus_minutes,
        score,
    }
}

/// Render the impact report as a compact text summary.
fn render_impact_summary(report: &CognitiveImpactReport) -> String {
    format!(
        "Anti-Interruption Score: {:.0}%\n\
         Interruptions prevented: {}\n\
         Focus time saved: {:.0} min\n\
         Echo tokens saved: {}\n\
         Cognitive load reduction: {:.0}%",
        report.score * 100.0,
        report.interruptions_prevented,
        report.focus_time_saved_minutes,
        report.echo_tokens_saved,
        report.cognitive_load_reduction * 100.0,
    )
}

#[cfg(test)]
mod tests {
    use super::{compute_impact, render_impact_summary};
    use crate::core::anti_interrupt::tracker::{
        InterruptionEvent, TEST_LOCK, record_interruption, reset_session,
    };

    #[test]
    fn empty_session_has_perfect_score() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();

        let report = compute_impact();
        assert_eq!(report.interruptions_prevented, 0);
        assert_eq!(report.score, 1.0);
        assert_eq!(report.cognitive_load_reduction, 0.0);
    }

    #[test]
    fn all_prevented_gives_perfect_score() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(InterruptionEvent::EchoRepetition { tokens: 50 }, true);
        record_interruption(
            InterruptionEvent::ContextSwitch {
                from: "parser".to_string(),
                to: "storage".to_string(),
            },
            true,
        );

        let report = compute_impact();
        assert_eq!(report.interruptions_prevented, 2);
        assert_eq!(report.context_switches_saved, 1);
        assert_eq!(report.echo_tokens_saved, 50);
        assert_eq!(report.focus_time_saved_minutes, 23.0);
        assert_eq!(report.score, 1.0);
    }

    #[test]
    fn mixed_events_calculates_correctly() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(InterruptionEvent::EchoRepetition { tokens: 80 }, true);
        record_interruption(
            InterruptionEvent::RedundantRead {
                path: "src/main.rs".to_string(),
            },
            false,
        );
        record_interruption(InterruptionEvent::BounceWaste { tokens: 10 }, true);

        let report = compute_impact();
        assert_eq!(report.interruptions_prevented, 2);
        assert!((report.score - (2.0 / 3.0)).abs() < f64::EPSILON);
        assert_eq!(report.cognitive_load_reduction, 0.04);
    }

    #[test]
    fn render_summary_format() {
        let _guard = TEST_LOCK.lock().expect("test lock should be available");
        reset_session();
        record_interruption(
            InterruptionEvent::ContextSwitch {
                from: "core".to_string(),
                to: "tests".to_string(),
            },
            true,
        );

        let summary = render_impact_summary(&compute_impact());
        assert_eq!(
            summary,
            "Anti-Interruption Score: 100%\n\
             Interruptions prevented: 1\n\
             Focus time saved: 23 min\n\
             Echo tokens saved: 0\n\
             Cognitive load reduction: 2%"
        );
    }
}
