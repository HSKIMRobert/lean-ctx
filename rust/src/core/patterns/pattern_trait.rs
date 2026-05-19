/// Plugin-ready trait for shell output compression patterns.
///
/// Each pattern matches a specific CLI tool (or family of tools) and
/// compresses its output into a token-efficient representation.
///
/// Existing patterns implement this via the blanket `compress(cmd, output)`
/// functions. Future plugins will register implementations dynamically.
pub trait CompressionPattern: Send + Sync {
    /// Human-readable name for this pattern (e.g. "cargo", "git", "docker").
    fn name(&self) -> &str;

    /// Version of this pattern's output format. Bump when the compressed
    /// output structure changes to maintain determinism guarantees.
    fn version(&self) -> u32 {
        1
    }

    /// Returns true if this pattern can handle the given command.
    /// Called during dispatch to find the appropriate pattern.
    fn matches(&self, command: &str) -> bool;

    /// Compress the shell output for the matched command.
    /// Returns `None` if the pattern cannot produce a shorter result.
    fn compress(&self, command: &str, output: &str) -> Option<String>;

    /// Command prefixes this pattern handles (for documentation/discovery).
    fn prefixes(&self) -> &[&str];
}

/// Metadata about a compression result, for observability and IR recording.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub pattern_name: String,
    pub pattern_version: u32,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub compressed: String,
}
