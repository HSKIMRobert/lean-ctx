//! Infinite post-Mythic prestige — "Ascension".
//!
//! Once a buddy reaches the final Mythic silhouette there is intentionally no
//! dead-end "MAX". Instead it ascends through an *unbounded* ladder of cosmic
//! ranks. Every tier changes the creature's title, aura colour and sparkle, and
//! there is always a next rank to chase — the companion never stops growing.
//!
//! Tier cost grows triangularly (`STEP_XP · n(n+1)/2`), so higher ranks take
//! longer yet remain forever reachable. Titles cycle through a cosmic list and
//! gain a roman-numeral suffix after each lap, so naming is endless too.

const MYTHIC_LEVEL: u32 = 65;
/// Base XP cost of the first ascension tier (grows per tier, see [`cumulative`]).
const STEP_XP: u64 = 30_000;

const TITLES: &[&str] = &[
    "Ascended",
    "Stellar",
    "Astral",
    "Cosmic",
    "Celestial",
    "Radiant",
    "Eternal",
    "Transcendent",
];

/// Bright 256-colour aura ramp; cycles so each ascension visibly recolours the
/// creature.
const COSMIC_COLORS: &[&str] = &[
    "\x1b[38;5;220m",
    "\x1b[38;5;51m",
    "\x1b[38;5;201m",
    "\x1b[38;5;213m",
    "\x1b[38;5;87m",
    "\x1b[38;5;229m",
    "\x1b[38;5;141m",
    "\x1b[38;5;159m",
];

/// XP needed just to enter Mythic territory (level 65) — the ascension origin.
fn mythic_entry_xp() -> u64 {
    u64::from(MYTHIC_LEVEL) * u64::from(MYTHIC_LEVEL) * 50
}

/// Cumulative XP beyond the Mythic entry required to *complete* tier `n`.
fn cumulative(n: u32) -> u64 {
    STEP_XP * u64::from(n) * (u64::from(n) + 1) / 2
}

/// Current ascension tier from total buddy XP. `0` means "not yet ascending".
pub(super) fn tier(xp: u64) -> u32 {
    let beyond = xp.saturating_sub(mythic_entry_xp());
    if beyond == 0 {
        return 0;
    }
    let ratio = beyond as f64 / STEP_XP as f64;
    (((1.0 + 8.0 * ratio).sqrt() - 1.0) / 2.0).floor() as u32
}

/// Progress (0.0..1.0) toward the next ascension tier — never reaches a hard cap.
pub(super) fn progress(xp: u64) -> f64 {
    let beyond = xp.saturating_sub(mythic_entry_xp());
    let t = tier(xp);
    let lo = cumulative(t);
    let hi = cumulative(t + 1);
    if hi <= lo {
        return 0.0;
    }
    (beyond.saturating_sub(lo) as f64 / (hi - lo) as f64).clamp(0.0, 1.0)
}

/// Cosmic rank title for a tier (cycles, with a roman-numeral suffix per lap).
pub(super) fn title(tier: u32) -> String {
    if tier == 0 {
        return "Mythic".to_string();
    }
    let idx = (tier - 1) as usize % TITLES.len();
    let lap = (tier - 1) as usize / TITLES.len();
    if lap == 0 {
        TITLES[idx].to_string()
    } else {
        format!("{} {}", TITLES[idx], roman(lap as u32 + 1))
    }
}

/// Aura colour for a tier; cycles so each rank looks distinct.
pub(super) fn color(tier: u32) -> &'static str {
    if tier == 0 {
        return "";
    }
    COSMIC_COLORS[(tier as usize - 1) % COSMIC_COLORS.len()]
}

fn roman(mut n: u32) -> String {
    const VALUES: &[(u32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for &(v, s) in VALUES {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_ascension_below_mythic() {
        assert_eq!(tier(0), 0);
        assert_eq!(tier(mythic_entry_xp()), 0);
    }

    #[test]
    fn tier_is_monotonic_and_endless() {
        let base = mythic_entry_xp();
        let mut last = 0;
        for mult in 1..50u64 {
            let t = tier(base + STEP_XP * mult * mult);
            assert!(t >= last, "tier must never decrease");
            last = t;
        }
        // Very large XP still yields a finite, larger tier — never a cap/panic.
        assert!(tier(base + 10_000_000_000) > last);
    }

    #[test]
    fn progress_stays_in_range() {
        let base = mythic_entry_xp();
        for extra in [0u64, 1, 15_000, 30_000, 250_000, 5_000_000] {
            let p = progress(base + extra);
            assert!((0.0..=1.0).contains(&p), "progress out of range: {p}");
        }
    }

    #[test]
    fn titles_cycle_with_roman_laps() {
        assert_eq!(title(0), "Mythic");
        assert_eq!(title(1), "Ascended");
        assert_eq!(title(TITLES.len() as u32), "Transcendent");
        // First title of the second lap gains a roman "II".
        assert_eq!(title(TITLES.len() as u32 + 1), "Ascended II");
    }

    #[test]
    fn roman_handles_large_numbers() {
        assert_eq!(roman(2), "II");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(2026), "MMXXVI");
    }
}
