//! Query-aware adaptive compression (#1312).
//!
//! Match compression level to task intent: exploration tasks get
//! aggressive compression, implementation tasks get full detail.
//! Based on SeleCom (arXiv 2602.15856): query-conditioned selective
//! compression outperforms full-context RAG.

/// Task intent classification for compression decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskIntent {
    /// Exploring / understanding the codebase. High compression is safe.
    Explore,
    /// Implementing / editing specific code. Full detail needed.
    Implement,
    /// Debugging a specific issue. Targeted detail needed.
    Debug,
    /// Reviewing code. Moderate compression acceptable.
    Review,
    /// Unknown intent. Default to moderate compression.
    Unknown,
}

impl TaskIntent {
    /// Classify intent from a task description string.
    pub(crate) fn classify(task: &str) -> Self {
        let lower = task.to_lowercase();

        if contains_any(
            &lower,
            &["implement", "add feature", "create", "build", "write"],
        ) {
            return Self::Implement;
        }
        if contains_any(
            &lower,
            &["fix", "debug", "error", "bug", "crash", "failing"],
        ) {
            return Self::Debug;
        }
        if contains_any(&lower, &["review", "audit", "check", "verify", "inspect"]) {
            return Self::Review;
        }
        if contains_any(
            &lower,
            &[
                "explore",
                "understand",
                "how does",
                "what is",
                "find",
                "search",
                "where",
            ],
        ) {
            return Self::Explore;
        }

        Self::Unknown
    }

    /// Recommended compression level for this intent.
    pub(crate) fn compression_level(&self) -> CompressionLevel {
        match self {
            TaskIntent::Explore => CompressionLevel::High,
            TaskIntent::Review | TaskIntent::Unknown => CompressionLevel::Medium,
            TaskIntent::Debug => CompressionLevel::Low,
            TaskIntent::Implement => CompressionLevel::Minimal,
        }
    }

    /// Suggested read mode for auto-mode resolution.
    pub(crate) fn suggested_read_mode(&self) -> &'static str {
        match self {
            TaskIntent::Explore => "map",
            TaskIntent::Review => "signatures",
            TaskIntent::Debug | TaskIntent::Implement => "full",
            TaskIntent::Unknown => "auto",
        }
    }
}

/// Compression intensity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CompressionLevel {
    /// No compression — full content.
    Minimal,
    /// Light compression (remove blanks, trailing whitespace).
    Low,
    /// Moderate compression (signatures + key implementations).
    Medium,
    /// Aggressive compression (outline only).
    High,
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_explore() {
        assert_eq!(
            TaskIntent::classify("how does the cache work?"),
            TaskIntent::Explore
        );
        assert_eq!(
            TaskIntent::classify("find the database module"),
            TaskIntent::Explore
        );
    }

    #[test]
    fn classify_implement() {
        assert_eq!(
            TaskIntent::classify("implement user authentication"),
            TaskIntent::Implement
        );
        assert_eq!(
            TaskIntent::classify("add feature for dark mode"),
            TaskIntent::Implement
        );
    }

    #[test]
    fn classify_debug() {
        assert_eq!(
            TaskIntent::classify("fix the null pointer error"),
            TaskIntent::Debug
        );
        assert_eq!(
            TaskIntent::classify("debug why tests are failing"),
            TaskIntent::Debug
        );
    }

    #[test]
    fn classify_review() {
        assert_eq!(
            TaskIntent::classify("review this pull request"),
            TaskIntent::Review
        );
    }

    #[test]
    fn classification_drives_compression() {
        assert_eq!(
            TaskIntent::Explore.compression_level(),
            CompressionLevel::High
        );
        assert_eq!(
            TaskIntent::Implement.compression_level(),
            CompressionLevel::Minimal
        );
        assert_eq!(TaskIntent::Debug.compression_level(), CompressionLevel::Low);
    }

    #[test]
    fn classification_drives_read_mode() {
        assert_eq!(TaskIntent::Explore.suggested_read_mode(), "map");
        assert_eq!(TaskIntent::Implement.suggested_read_mode(), "full");
        assert_eq!(TaskIntent::Debug.suggested_read_mode(), "full");
    }
}
