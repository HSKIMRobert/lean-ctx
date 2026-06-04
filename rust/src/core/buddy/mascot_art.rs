//! The lean-ctx mascot — a single iconic "Pixel Sprite" rendered in retro
//! half-block pixel art, with five evolution stages (Egg → Hatchling → Teen →
//! Guardian → Ascended).
//!
//! There is exactly ONE creature. A user's dominant language maps to an
//! *element* (handled in [`super::types::Species`]) that only changes the
//! mascot's colour and title — never its silhouette. This keeps the brand
//! single and recognisable (think Ferris/Gopher/Octocat) while still feeling
//! personal.
//!
//! Eye placeholders `{L}` / `{R}` are replaced with mood-dependent eyes. All
//! glyphs are deliberately terminal-width-1 (no emoji-presentation chars) so the
//! sprite never overflows its frame.

use super::evolution::EvolutionStage;
use super::types::Mood;

/// Fixed render width of the sprite block.
const W: usize = 18;

pub(super) fn width() -> usize {
    W
}

/// Frame interval in milliseconds; evolved forms animate faster for liveliness.
pub(super) fn anim_ms_for(stage: &EvolutionStage) -> u32 {
    match stage {
        EvolutionStage::Egg => 1100,
        EvolutionStage::Baby => 900,
        EvolutionStage::Teen => 720,
        EvolutionStage::Adult => 560,
        EvolutionStage::Mythic => 420,
    }
}

/// Mood → (left eye, right eye). All width-1.
fn mood_eyes(mood: &Mood) -> (&'static str, &'static str) {
    match mood {
        Mood::Ecstatic => ("◕", "◕"),
        Mood::Happy => ("●", "●"),
        Mood::Content => ("o", "o"),
        Mood::Worried => (">", "<"),
        Mood::Sleeping => ("-", "-"),
    }
}

/// The static, mood-aware sprite for an evolution stage, block-centered to [`W`].
pub(super) fn sprite_for(stage: &EvolutionStage, mood: &Mood) -> Vec<String> {
    let (l, r) = mood_eyes(mood);
    compose(raw_lines(stage), l, r)
}

/// Animation frames for the web dashboard / TUI: idle, blink, and (for evolved
/// stages) a sparkle pulse.
pub(super) fn frames_for(stage: &EvolutionStage, mood: &Mood) -> Vec<Vec<String>> {
    let mut frames = vec![sprite_for(stage, mood)];
    // Blink: briefly close the eyes.
    frames.push(compose(raw_lines(stage), "-", "-"));
    // Evolved forms get an extra sparkle frame for a lively shimmer.
    if matches!(stage, EvolutionStage::Adult | EvolutionStage::Mythic) {
        let (l, r) = mood_eyes(mood);
        frames.push(compose_sparkle(raw_lines(stage), l, r));
    }
    frames
}

/// Replace eye placeholders and block-center the whole sprite so that every line
/// shares the same left margin (preserving the authored vertical alignment of
/// eyes, mouth and core).
fn compose(lines: &[&str], l: &str, r: &str) -> Vec<String> {
    let subbed: Vec<String> = lines
        .iter()
        .map(|s| s.replace("{L}", l).replace("{R}", r))
        .collect();
    block_center(&subbed)
}

/// Like [`compose`], but swaps the calm aura dots for bright sparkles.
fn compose_sparkle(lines: &[&str], l: &str, r: &str) -> Vec<String> {
    let subbed: Vec<String> = lines
        .iter()
        .map(|s| {
            s.replace("{L}", l)
                .replace("{R}", r)
                .replace('✦', "✧")
                .replace('◆', "◇")
        })
        .collect();
    block_center(&subbed)
}

fn block_center(lines: &[String]) -> Vec<String> {
    use super::super::theme::{pad_right, visual_len};
    let max_w = lines.iter().map(|l| visual_len(l)).max().unwrap_or(0);
    let pad = W.saturating_sub(max_w) / 2;
    let margin = " ".repeat(pad);
    lines
        .iter()
        .map(|l| pad_right(&format!("{margin}{l}"), W))
        .collect()
}

/// The authored pixel art per stage. Lines keep their relative leading spaces;
/// [`block_center`] applies a single uniform shift so columns stay aligned.
fn raw_lines(stage: &EvolutionStage) -> &'static [&'static str] {
    match stage {
        // ── EGG ── a cracking pixel egg, no eyes yet ──────────────
        EvolutionStage::Egg => &[
            r" ▗▄▄▄▄▖ ",
            r"▟██████▙",
            r"███▘▝███",
            r"████████",
            r"▜██████▛",
            r" ▀▀▀▀▀▀ ",
        ],

        // ── HATCHLING (Baby) ── tiny round critter, big eyes ──────
        EvolutionStage::Baby => &[
            r"▗▖    ▗▖",
            r"▟█▀▀▀▀█▙",
            r"█ {L}  {R} █",
            r"█  ▿▿  █",
            r"▜█▄▄▄▄█▛",
            r" ▝▘  ▝▘ ",
        ],

        // ── TEEN ── taller, pointed ears, little arms ─────────────
        EvolutionStage::Teen => &[
            r"▟▙      ▟▙",
            r"▐█▀▀▀▀▀▀█▌",
            r"▐█ {L}  {R} █▌",
            r"▐█  ▿▿  █▌",
            r"▟██▄▄▄▄██▙",
            r"▝█      █▘",
            r" ▝▀▄▄▄▄▀▘ ",
            r"  ▝▘  ▝▘  ",
        ],

        // ── GUARDIAN (Adult) ── crest, broad body, a power core ───
        EvolutionStage::Adult => &[
            r" ▟▙ ▟██▙ ▟▙ ",
            r"▐█▀▀▀▀▀▀▀▀█▌",
            r"▐█ {L}    {R} █▌",
            r"▐█   ▿▿   █▌",
            r"▐██▄▄◆◆▄▄██▌",
            r"▝██      ██▘",
            r" ▝▀▄▄▄▄▄▄▀▘ ",
            r"   ▝█  █▘   ",
        ],

        // ── ASCENDED (Mythic) ── winged, crowned, radiant aura ────
        EvolutionStage::Mythic => &[
            r"✦  ▟█◆█▙   ✦",
            r" ▚▟▀▀▀▀▀▀▙▞ ",
            r"▟█ {L}    {R} █▙",
            r"██   ▿▿   ██",
            r"▜██▄◆◆◆◆▄██▛",
            r" ▝██    ██▘ ",
            r"✦ ▝▀▄▄▄▄▀▘ ✦",
            r"   ▝█ █▘    ",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stage_renders_at_fixed_width() {
        let stages = [
            EvolutionStage::Egg,
            EvolutionStage::Baby,
            EvolutionStage::Teen,
            EvolutionStage::Adult,
            EvolutionStage::Mythic,
        ];
        for st in &stages {
            for mood in &[Mood::Ecstatic, Mood::Worried, Mood::Sleeping] {
                let sprite = sprite_for(st, mood);
                assert!(!sprite.is_empty(), "{st:?} produced no lines");
                for line in &sprite {
                    assert_eq!(
                        super::super::super::theme::visual_len(line),
                        W,
                        "{st:?}/{mood:?} line not width {W}: {line:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn eyes_are_injected() {
        let sprite = sprite_for(&EvolutionStage::Baby, &Mood::Happy);
        assert!(sprite.join("\n").contains('●'), "happy eyes missing");
    }

    #[test]
    fn frames_present_for_all_stages() {
        for st in &[
            EvolutionStage::Egg,
            EvolutionStage::Baby,
            EvolutionStage::Adult,
            EvolutionStage::Mythic,
        ] {
            assert!(frames_for(st, &Mood::Happy).len() >= 2);
        }
    }
}
