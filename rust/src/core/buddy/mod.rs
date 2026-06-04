pub mod achievements;
mod ascension;
pub mod evolution;
mod format;
mod mascot_art;
mod rpg;
mod sprite;
mod types;

pub use evolution::EvolutionStage;
pub use format::{format_buddy_block, format_buddy_block_at, format_buddy_full};
pub use types::{BuddyState, Mood, Rarity, Species};

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
    fn xp_next_level_increases() {
        let lv1 = (1u64 + 1) * (1 + 1) * 50;
        let lv10 = (10u64 + 1) * (10 + 1) * 50;
        assert!(lv10 > lv1);
    }
}
