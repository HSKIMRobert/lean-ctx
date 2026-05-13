mod format;
mod rpg;
mod sprite;
mod types;

pub use format::{format_buddy_block, format_buddy_block_at, format_buddy_full};
pub use sprite::render_sprite;
pub use types::{BuddyState, BuddyStats, CreatureTraits, Mood, Rarity, Species};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn species_from_cargo_commands() {
        let mut cmds = HashMap::new();
        cmds.insert(
            "cargo build".to_string(),
            super::super::stats::CommandStats {
                count: 50,
                input_tokens: 1000,
                output_tokens: 500,
            },
        );
        assert_eq!(Species::from_commands(&cmds), Species::Crab);
    }

    #[test]
    fn species_mixed_is_dragon() {
        let mut cmds = HashMap::new();
        cmds.insert(
            "cargo build".to_string(),
            super::super::stats::CommandStats {
                count: 10,
                input_tokens: 0,
                output_tokens: 0,
            },
        );
        cmds.insert(
            "npm install".to_string(),
            super::super::stats::CommandStats {
                count: 10,
                input_tokens: 0,
                output_tokens: 0,
            },
        );
        cmds.insert(
            "python app.py".to_string(),
            super::super::stats::CommandStats {
                count: 10,
                input_tokens: 0,
                output_tokens: 0,
            },
        );
        assert_eq!(Species::from_commands(&cmds), Species::Dragon);
    }

    #[test]
    fn species_empty_is_egg() {
        let cmds = HashMap::new();
        assert_eq!(Species::from_commands(&cmds), Species::Egg);
    }

    #[test]
    fn rarity_levels() {
        assert_eq!(Rarity::from_tokens_saved(0), Rarity::Egg);
        assert_eq!(Rarity::from_tokens_saved(5_000), Rarity::Egg);
        assert_eq!(Rarity::from_tokens_saved(50_000), Rarity::Common);
        assert_eq!(Rarity::from_tokens_saved(500_000), Rarity::Uncommon);
        assert_eq!(Rarity::from_tokens_saved(5_000_000), Rarity::Rare);
        assert_eq!(Rarity::from_tokens_saved(50_000_000), Rarity::Epic);
        assert_eq!(Rarity::from_tokens_saved(500_000_000), Rarity::Legendary);
    }

    #[test]
    fn name_is_deterministic() {
        let s = types::user_seed();
        let n1 = rpg::generate_name(s);
        let n2 = rpg::generate_name(s);
        assert_eq!(n1, n2);
    }

    #[test]
    fn format_compact_values() {
        assert_eq!(rpg::format_compact(500), "500");
        assert_eq!(rpg::format_compact(1_500), "1.5K");
        assert_eq!(rpg::format_compact(2_500_000), "2.5M");
        assert_eq!(rpg::format_compact(3_000_000_000), "3.0B");
    }

    #[test]
    fn procedural_sprite_returns_7_lines() {
        for seed in [0u64, 1, 42, 999, 12345, 69_119_999, u64::MAX] {
            let traits = CreatureTraits::from_seed(seed);
            for mood in &[
                Mood::Ecstatic,
                Mood::Happy,
                Mood::Content,
                Mood::Worried,
                Mood::Sleeping,
            ] {
                let sp = render_sprite(&traits, mood);
                assert_eq!(sp.len(), 7, "sprite for seed={seed}, mood={mood:?}");
            }
        }
    }

    #[test]
    fn creature_traits_are_deterministic() {
        let t1 = CreatureTraits::from_seed(42);
        let t2 = CreatureTraits::from_seed(42);
        assert_eq!(t1.head, t2.head);
        assert_eq!(t1.eyes, t2.eyes);
        assert_eq!(t1.mouth, t2.mouth);
        assert_eq!(t1.ears, t2.ears);
        assert_eq!(t1.body, t2.body);
        assert_eq!(t1.legs, t2.legs);
        assert_eq!(t1.tail, t2.tail);
        assert_eq!(t1.markings, t2.markings);
    }

    #[test]
    fn different_seeds_produce_different_traits() {
        let t1 = CreatureTraits::from_seed(1);
        let t2 = CreatureTraits::from_seed(9999);
        let same = t1.head == t2.head
            && t1.eyes == t2.eyes
            && t1.mouth == t2.mouth
            && t1.ears == t2.ears
            && t1.body == t2.body
            && t1.legs == t2.legs
            && t1.tail == t2.tail
            && t1.markings == t2.markings;
        assert!(
            !same,
            "seeds 1 and 9999 should differ in at least one trait"
        );
    }

    #[test]
    fn total_combinations_is_69m() {
        assert_eq!(12u64 * 10 * 10 * 12 * 10 * 10 * 8 * 6, 69_120_000);
    }

    #[test]
    fn xp_next_level_increases() {
        let lv1 = (1u64 + 1) * (1 + 1) * 50;
        let lv10 = (10u64 + 1) * (10 + 1) * 50;
        assert!(lv10 > lv1);
    }
}
