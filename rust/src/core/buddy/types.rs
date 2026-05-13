use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Species {
    Egg,
    Crab,
    Snake,
    Owl,
    Gopher,
    Whale,
    Fox,
    Dragon,
}

impl Species {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Egg => "Egg",
            Self::Crab => "Crab",
            Self::Snake => "Snake",
            Self::Owl => "Owl",
            Self::Gopher => "Gopher",
            Self::Whale => "Whale",
            Self::Fox => "Fox",
            Self::Dragon => "Dragon",
        }
    }

    pub fn from_commands(commands: &HashMap<String, super::super::stats::CommandStats>) -> Self {
        let mut scores: HashMap<&str, u64> = HashMap::new();

        for (cmd, stats) in commands {
            let lang = classify_command(cmd);
            if !lang.is_empty() {
                *scores.entry(lang).or_default() += stats.count;
            }
        }

        if scores.is_empty() {
            return Self::Egg;
        }

        let total: u64 = scores.values().sum();
        let (top_lang, top_count) = scores
            .iter()
            .max_by_key(|(_, c)| **c)
            .map_or(("", 0), |(l, c)| (*l, *c));

        let dominance = top_count as f64 / total as f64;

        if dominance < 0.4 {
            return Self::Dragon;
        }

        match top_lang {
            "rust" => Self::Crab,
            "python" => Self::Snake,
            "js" => Self::Owl,
            "go" => Self::Gopher,
            "docker" => Self::Whale,
            "git" => Self::Fox,
            _ => Self::Dragon,
        }
    }
}

fn classify_command(cmd: &str) -> &'static str {
    let lower = cmd.to_lowercase();
    if lower.starts_with("cargo") || lower.starts_with("rustc") {
        "rust"
    } else if lower.starts_with("python")
        || lower.starts_with("pip")
        || lower.starts_with("uv ")
        || lower.starts_with("pytest")
        || lower.starts_with("ruff")
    {
        "python"
    } else if lower.starts_with("npm")
        || lower.starts_with("pnpm")
        || lower.starts_with("yarn")
        || lower.starts_with("tsc")
        || lower.starts_with("jest")
        || lower.starts_with("vitest")
        || lower.starts_with("node")
        || lower.starts_with("bun")
    {
        "js"
    } else if lower.starts_with("go ") {
        "go"
    } else if lower.starts_with("docker") || lower.starts_with("kubectl") {
        "docker"
    } else if lower.starts_with("git ") {
        "git"
    } else {
        ""
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Rarity {
    Egg,
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn from_tokens_saved(saved: u64) -> Self {
        match saved {
            0..=9_999 => Self::Egg,
            10_000..=99_999 => Self::Common,
            100_000..=999_999 => Self::Uncommon,
            1_000_000..=9_999_999 => Self::Rare,
            10_000_000..=99_999_999 => Self::Epic,
            _ => Self::Legendary,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Egg => "Egg",
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Egg | Self::Common => "\x1b[37m",
            Self::Uncommon => "\x1b[32m",
            Self::Rare => "\x1b[34m",
            Self::Epic => "\x1b[35m",
            Self::Legendary => "\x1b[33m",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Mood {
    Ecstatic,
    Happy,
    Content,
    Worried,
    Sleeping,
}

impl Mood {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ecstatic => "Ecstatic",
            Self::Happy => "Happy",
            Self::Content => "Content",
            Self::Worried => "Worried",
            Self::Sleeping => "Sleeping",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ecstatic => "*_*",
            Self::Happy => "o_o",
            Self::Content => "-_-",
            Self::Worried => ">_<",
            Self::Sleeping => "u_u",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddyStats {
    pub compression: u8,
    pub vigilance: u8,
    pub endurance: u8,
    pub wisdom: u8,
    pub experience: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureTraits {
    pub head: u8,
    pub eyes: u8,
    pub mouth: u8,
    pub ears: u8,
    pub body: u8,
    pub legs: u8,
    pub tail: u8,
    pub markings: u8,
}

impl CreatureTraits {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            head: (seed % 12) as u8,
            eyes: ((seed / 12) % 10) as u8,
            mouth: ((seed / 120) % 10) as u8,
            ears: ((seed / 1_200) % 12) as u8,
            body: ((seed / 14_400) % 10) as u8,
            legs: ((seed / 144_000) % 10) as u8,
            tail: ((seed / 1_440_000) % 8) as u8,
            markings: ((seed / 11_520_000) % 6) as u8,
        }
    }
}

pub(super) fn user_seed() -> u64 {
    dirs::home_dir().map_or(42, |p| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        p.hash(&mut h);
        h.finish()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddyState {
    pub name: String,
    pub species: Species,
    pub rarity: Rarity,
    pub level: u32,
    pub xp: u64,
    pub xp_next_level: u64,
    pub mood: Mood,
    pub stats: BuddyStats,
    pub speech: String,
    pub tokens_saved: u64,
    pub bugs_prevented: u64,
    pub streak_days: u32,
    pub ascii_art: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ascii_frames: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anim_ms: Option<u32>,
    pub traits: CreatureTraits,
}

impl BuddyState {
    pub fn compute() -> Self {
        let store = super::super::stats::load();
        let tokens_saved = store
            .total_input_tokens
            .saturating_sub(store.total_output_tokens);

        let project_root = super::format::detect_project_root_for_buddy();
        let gotcha_store = if project_root.is_empty() {
            super::super::gotcha_tracker::GotchaStore::new("none")
        } else {
            super::super::gotcha_tracker::GotchaStore::load(&project_root)
        };

        let bugs_prevented = gotcha_store.stats.total_prevented;
        let errors_detected = gotcha_store.stats.total_errors_detected;

        let species = Species::from_commands(&store.commands);
        let rarity = Rarity::from_tokens_saved(tokens_saved);

        let xp = tokens_saved / 1000 + store.total_commands * 5 + bugs_prevented * 100;
        let level = ((xp as f64 / 50.0).sqrt().floor() as u32).min(99);
        let xp_next_level = ((level + 1) as u64) * ((level + 1) as u64) * 50;

        let streak_days = super::rpg::compute_streak(&store.daily);
        let compression_rate = if store.total_input_tokens > 0 {
            (tokens_saved as f64 / store.total_input_tokens as f64 * 100.0) as u8
        } else {
            0
        };

        let mood = super::rpg::compute_mood(
            compression_rate,
            errors_detected,
            bugs_prevented,
            streak_days,
            &store,
        );

        let rpg_stats = super::rpg::compute_rpg_stats(
            compression_rate,
            bugs_prevented,
            errors_detected,
            streak_days,
            store.commands.len(),
            store.total_commands,
        );

        let seed = user_seed();
        let traits = CreatureTraits::from_seed(seed);
        let name = super::rpg::generate_name(seed);
        let sprite = super::sprite::render_sprite_pack(&traits, &mood, level);
        let ascii_art = sprite.base.clone();
        let speech = super::rpg::generate_speech(&mood, tokens_saved, bugs_prevented, streak_days);

        Self {
            name,
            species,
            rarity,
            level,
            xp,
            xp_next_level,
            mood,
            stats: rpg_stats,
            speech,
            tokens_saved,
            bugs_prevented,
            streak_days,
            ascii_art,
            ascii_frames: sprite.frames,
            anim_ms: sprite.anim_ms,
            traits,
        }
    }
}
