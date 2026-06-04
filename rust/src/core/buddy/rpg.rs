use super::types::Mood;

pub(super) fn compute_mood(
    compression: u8,
    errors: u64,
    prevented: u64,
    streak: u32,
    store: &super::super::stats::StatsStore,
) -> Mood {
    let hours_since_last = store
        .last_use
        .as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map_or(999, |dt| {
            (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours()
        });

    if hours_since_last > 48 {
        return Mood::Sleeping;
    }

    let recent_errors = store
        .daily
        .iter()
        .rev()
        .take(1)
        .any(|d| d.input_tokens > 0 && d.output_tokens > d.input_tokens);

    if compression > 60 && errors == 0 && streak >= 7 {
        Mood::Ecstatic
    } else if compression > 40 || prevented > 0 {
        Mood::Happy
    } else if recent_errors || (errors > 5 && prevented == 0) {
        Mood::Worried
    } else {
        Mood::Content
    }
}

pub(super) fn compute_streak(daily: &[super::super::stats::DayStats]) -> u32 {
    if daily.is_empty() {
        return 0;
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut streak = 0u32;
    let mut expected = today.clone();

    for day in daily.iter().rev() {
        if day.date == expected && day.commands > 0 {
            streak += 1;
            if let Ok(dt) = chrono::NaiveDate::parse_from_str(&expected, "%Y-%m-%d") {
                expected = (dt - chrono::Duration::days(1))
                    .format("%Y-%m-%d")
                    .to_string();
            } else {
                break;
            }
        } else if day.date < expected {
            break;
        }
    }
    streak
}

pub(super) fn generate_name(seed: u64) -> String {
    const ADJ: &[&str] = &[
        "Swift", "Quiet", "Bright", "Bold", "Clever", "Brave", "Lucky", "Tiny", "Cosmic", "Fuzzy",
        "Nimble", "Jolly", "Mighty", "Gentle", "Witty", "Keen", "Sly", "Calm", "Wild", "Vivid",
        "Dusk", "Dawn", "Neon", "Frost", "Solar", "Lunar", "Pixel", "Turbo", "Nano", "Mega",
    ];
    const NOUN: &[&str] = &[
        "Ember", "Reef", "Spark", "Byte", "Flux", "Echo", "Drift", "Glitch", "Pulse", "Shade",
        "Orbit", "Fern", "Rust", "Zinc", "Flint", "Quartz", "Maple", "Cedar", "Opal", "Moss",
        "Ridge", "Cove", "Peak", "Dune", "Vale", "Brook", "Cliff", "Storm", "Blaze", "Mist",
    ];

    let adj_idx = (seed >> 8) as usize % ADJ.len();
    let noun_idx = (seed >> 16) as usize % NOUN.len();
    format!("{} {}", ADJ[adj_idx], NOUN[noun_idx])
}

pub(super) fn generate_speech(
    mood: &Mood,
    tokens_saved: u64,
    bugs_prevented: u64,
    streak: u32,
) -> String {
    let today_saved = today_tokens_saved();
    match mood {
        Mood::Ecstatic => {
            if streak >= 30 {
                format!("{streak}-day streak! You're a legend!")
            } else if bugs_prevented > 10 {
                format!("{bugs_prevented} bugs squashed! Nothing gets past us!")
            } else if today_saved > 1_000_000 {
                format!(
                    "{} saved just today! Incredible!",
                    format_compact(today_saved)
                )
            } else {
                format!(
                    "{} total tokens saved! On fire!",
                    format_compact(tokens_saved)
                )
            }
        }
        Mood::Happy => {
            if streak >= 7 {
                format!("{streak}-day streak! We're in the zone!")
            } else if today_saved > 100_000 {
                format!(
                    "{} saved today — great session!",
                    format_compact(today_saved)
                )
            } else if bugs_prevented > 0 {
                format!("Caught {bugs_prevented} bugs before they happened!")
            } else {
                format!("{} tokens saved so far!", format_compact(tokens_saved))
            }
        }
        Mood::Content => {
            if today_saved > 0 {
                format!(
                    "{} saved today. Steady progress.",
                    format_compact(today_saved)
                )
            } else {
                "Watching your code... all good.".to_string()
            }
        }
        Mood::Worried => {
            if streak == 0 {
                "Haven't seen you in a while. Let's code!".to_string()
            } else {
                "I see some errors. Let's fix them together!".to_string()
            }
        }
        Mood::Sleeping => "Zzz... wake me with some code!".to_string(),
    }
}

fn today_tokens_saved() -> u64 {
    let store = super::super::stats::load();
    store
        .daily
        .last()
        .filter(|d| d.date == chrono::Local::now().format("%Y-%m-%d").to_string())
        .map_or(0, |d| d.input_tokens.saturating_sub(d.output_tokens))
}

pub(super) fn format_compact(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}
