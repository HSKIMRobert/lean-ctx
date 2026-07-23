use serde::{Deserialize, Serialize};

/// A complete context profile definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub profile: ProfileMeta,
    #[serde(default)]
    pub read: ReadConfig,
    #[serde(default)]
    pub compression: CompressionConfig,
    #[serde(default)]
    pub translation: TranslationConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub memory: crate::core::memory_policy::MemoryPolicyOverrides,
    #[serde(default)]
    pub verification: crate::core::output_verification::VerificationConfig,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub pipeline: PipelineConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub degradation: DegradationConfig,
    #[serde(default)]
    pub autonomy: ProfileAutonomy,
    #[serde(default)]
    pub output_hints: OutputHints,
}

/// Profile identity and inheritance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileMeta {
    #[serde(default)]
    pub name: String,
    pub inherits: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// Read behavior configuration.
///
/// Fields are `Option<T>` for field-level profile inheritance.
/// Use `_effective()` methods to get the resolved value with defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReadConfig {
    pub default_mode: Option<String>,
    pub max_tokens_per_file: Option<usize>,
    pub prefer_cache: Option<bool>,
}

impl ReadConfig {
    pub fn default_mode_effective(&self) -> &str {
        self.default_mode.as_deref().unwrap_or("auto")
    }
    pub fn max_tokens_per_file_effective(&self) -> usize {
        self.max_tokens_per_file.unwrap_or(50_000)
    }
    pub fn prefer_cache_effective(&self) -> bool {
        self.prefer_cache.unwrap_or(false)
    }
}

/// Compression strategy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CompressionConfig {
    /// Enable adaptive compression depth (#1195). When true, compression
    /// aggressiveness is reduced dynamically based on bounce rate and session
    /// length. Default: true.
    pub adaptive: Option<bool>,
    pub crp_mode: Option<String>,
    pub output_density: Option<String>,
    pub entropy_threshold: Option<f64>,
    pub terse_mode: Option<bool>,
}

impl CompressionConfig {
    pub fn crp_mode_effective(&self) -> &str {
        self.crp_mode.as_deref().unwrap_or("tdd")
    }
    pub fn output_density_effective(&self) -> &str {
        self.output_density.as_deref().unwrap_or("normal")
    }
    pub fn entropy_threshold_effective(&self) -> f64 {
        self.entropy_threshold.unwrap_or(0.3)
    }
    pub fn terse_mode_effective(&self) -> bool {
        self.terse_mode.unwrap_or(false)
    }
    pub fn adaptive_effective(&self) -> bool {
        self.adaptive.unwrap_or(true)
    }
}

/// Translation (tokenizer-aware) configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TranslationConfig {
    /// If false, preserve legacy CRP/TDD formats without post-translation.
    pub enabled: Option<bool>,
    /// legacy|ascii|auto
    pub ruleset: Option<String>,
}

impl TranslationConfig {
    pub fn enabled_effective(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    pub fn ruleset_effective(&self) -> &str {
        self.ruleset.as_deref().unwrap_or("legacy")
    }
}

/// Layout (attention-aware reorder) configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LayoutConfig {
    /// If false, preserve original order.
    pub enabled: Option<bool>,
    /// Minimum line count for enabling reorder.
    pub min_lines: Option<usize>,
}

impl LayoutConfig {
    pub fn enabled_effective(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    pub fn min_lines_effective(&self) -> usize {
        self.min_lines.unwrap_or(15)
    }
}

/// Routing policy overrides (intent → model tier → read mode/budgets).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Hard cap for recommended model tier: fast|standard|premium.
    #[serde(default)]
    pub max_model_tier: Option<String>,
    /// If true, apply deterministic routing degradation under budget/pressure.
    #[serde(default)]
    pub degrade_under_pressure: Option<bool>,
}

impl RoutingConfig {
    pub fn max_model_tier_effective(&self) -> &str {
        self.max_model_tier.as_deref().unwrap_or("premium")
    }

    pub fn degrade_under_pressure_effective(&self) -> bool {
        self.degrade_under_pressure.unwrap_or(true)
    }
}

/// Budget/SLO degradation policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DegradationConfig {
    /// If true, enforce throttling/blocking decisions. Default is warn-only.
    #[serde(default)]
    pub enforce: Option<bool>,
    /// Throttle duration (ms) when policy verdict is Throttle. Default: 250ms.
    #[serde(default)]
    pub throttle_ms: Option<u64>,
}

impl DegradationConfig {
    pub fn enforce_effective(&self) -> bool {
        self.enforce.unwrap_or(false)
    }

    pub fn throttle_ms_effective(&self) -> u64 {
        self.throttle_ms.unwrap_or(250)
    }
}

/// Controls which optional hints/footers are appended to tool output.
/// All default to `false` for minimal output overhead.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputHints {
    pub compressed_hint: Option<bool>,
    pub archive_hint: Option<bool>,
    pub verify_footer: Option<bool>,
    pub related_hint: Option<bool>,
    pub semantic_hint: Option<bool>,
    pub elicitation_hint: Option<bool>,
    pub checkpoint_in_output: Option<bool>,
    pub graph_context_block: Option<bool>,
    pub efficiency_hint: Option<bool>,
}

impl OutputHints {
    pub fn compressed_hint(&self) -> bool {
        self.compressed_hint.unwrap_or(false)
    }
    pub fn archive_hint(&self) -> bool {
        self.archive_hint.unwrap_or(false)
    }
    pub fn verify_footer(&self) -> bool {
        self.verify_footer.unwrap_or(false)
    }
    pub fn related_hint(&self) -> bool {
        self.related_hint.unwrap_or(false)
    }
    pub fn semantic_hint(&self) -> bool {
        self.semantic_hint.unwrap_or(false)
    }
    pub fn elicitation_hint(&self) -> bool {
        self.elicitation_hint.unwrap_or(false)
    }
    pub fn checkpoint_in_output(&self) -> bool {
        self.checkpoint_in_output.unwrap_or(false)
    }
    pub fn graph_context_block(&self) -> bool {
        self.graph_context_block.unwrap_or(false)
    }
    pub fn efficiency_hint(&self) -> bool {
        self.efficiency_hint.unwrap_or(false)
    }
}

/// Token and cost budget limits.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BudgetConfig {
    pub max_context_tokens: Option<usize>,
    pub max_shell_invocations: Option<usize>,
    pub max_cost_usd: Option<f64>,
}

impl BudgetConfig {
    pub fn max_context_tokens_effective(&self) -> usize {
        self.max_context_tokens.unwrap_or(200_000)
    }
    pub fn max_shell_invocations_effective(&self) -> usize {
        self.max_shell_invocations.unwrap_or(100)
    }
    pub fn max_cost_usd_effective(&self) -> f64 {
        self.max_cost_usd.unwrap_or(5.0)
    }
}

/// Pipeline layer activation per profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PipelineConfig {
    pub intent: Option<bool>,
    pub relevance: Option<bool>,
    pub compression: Option<bool>,
    pub translation: Option<bool>,
}

impl PipelineConfig {
    pub fn intent_effective(&self) -> bool {
        self.intent.unwrap_or(true)
    }
    pub fn relevance_effective(&self) -> bool {
        self.relevance.unwrap_or(true)
    }
    pub fn compression_effective(&self) -> bool {
        self.compression.unwrap_or(true)
    }
    pub fn translation_effective(&self) -> bool {
        self.translation.unwrap_or(true)
    }
}

/// Autonomy overrides per profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProfileAutonomy {
    pub enabled: Option<bool>,
    pub auto_preload: Option<bool>,
    pub auto_dedup: Option<bool>,
    pub auto_related: Option<bool>,
    pub silent_preload: Option<bool>,
    /// Enable bounded prefetch after reads (opt-in by default).
    pub auto_prefetch: Option<bool>,
    /// Enable response shaping for large outputs (opt-in by default).
    pub auto_response: Option<bool>,
    pub dedup_threshold: Option<usize>,
    pub prefetch_max_files: Option<usize>,
    pub prefetch_budget_tokens: Option<usize>,
    pub response_min_tokens: Option<usize>,
    pub checkpoint_interval: Option<u32>,
}

impl ProfileAutonomy {
    pub fn enabled_effective(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    pub fn auto_preload_effective(&self) -> bool {
        self.auto_preload.unwrap_or(true)
    }
    pub fn auto_dedup_effective(&self) -> bool {
        self.auto_dedup.unwrap_or(true)
    }
    pub fn auto_related_effective(&self) -> bool {
        self.auto_related.unwrap_or(true)
    }
    pub fn silent_preload_effective(&self) -> bool {
        self.silent_preload.unwrap_or(true)
    }
    pub fn auto_prefetch_effective(&self) -> bool {
        self.auto_prefetch.unwrap_or(false)
    }
    pub fn auto_response_effective(&self) -> bool {
        self.auto_response.unwrap_or(false)
    }
    pub fn dedup_threshold_effective(&self) -> usize {
        self.dedup_threshold.unwrap_or(8)
    }
    pub fn prefetch_max_files_effective(&self) -> usize {
        self.prefetch_max_files.unwrap_or(3)
    }
    pub fn prefetch_budget_tokens_effective(&self) -> usize {
        self.prefetch_budget_tokens.unwrap_or(4000)
    }
    pub fn response_min_tokens_effective(&self) -> usize {
        self.response_min_tokens.unwrap_or(600)
    }
    pub fn checkpoint_interval_effective(&self) -> u32 {
        self.checkpoint_interval.unwrap_or(15)
    }
}
