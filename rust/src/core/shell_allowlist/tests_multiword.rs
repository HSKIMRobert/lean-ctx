//! Multi-word allowlist entry tests (GH #1419).
//!
//! Extracted from `tests.rs` to stay within the 1500-line LOC gate.

use super::enforcement::{check_all_segments, matches_allowlist_entry};
use super::tests::allow;

#[test]
fn allowlist_matches_first_word_of_multi_word_entry() {
    let allowlist = vec!["terraform plan *".to_string()];
    assert!(
        matches_allowlist_entry("terraform", &allowlist),
        "first word of 'terraform plan *' must match base 'terraform'"
    );
}

#[test]
fn allowlist_exact_match_still_works() {
    let allowlist = vec!["git".to_string(), "cargo".to_string()];
    assert!(matches_allowlist_entry("git", &allowlist));
    assert!(matches_allowlist_entry("cargo", &allowlist));
    assert!(!matches_allowlist_entry("terraform", &allowlist));
}

#[test]
fn allowlist_multi_word_does_not_false_positive() {
    let allowlist = vec!["terraform plan *".to_string()];
    assert!(
        !matches_allowlist_entry("git", &allowlist),
        "unrelated base must not match 'terraform plan *'"
    );
}

// GH #1419: end-to-end integration tests via enforce_shell_allowlist.
// These exercise the full allowlist pipeline (not just matches_allowlist_entry)
// so a regression in check_all_segments or check_interpreter_inner is caught.

#[test]
fn issue_1419_multiword_extra_allows_direct_command() {
    let list = allow(&["git", "ls", "terraform plan *"]);
    assert!(
        check_all_segments("terraform plan -no-color -lock=false", &list).is_ok(),
        "terraform plan with multi-word allowlist entry must be allowed"
    );
}

#[test]
fn issue_1419_multiword_extra_blocks_unlisted_binary() {
    let list = allow(&["terraform plan *"]);
    let result = check_all_segments("kubectl get pods", &list);
    assert!(result.is_err(), "kubectl must still be blocked");
    assert!(result.unwrap_err().contains("kubectl"));
}

#[test]
fn issue_1419_multiword_entry_via_delegation_wrapper() {
    let list = allow(&["timeout", "terraform plan *"]);
    assert!(
        check_all_segments("timeout 120 terraform plan -no-color -lock=false", &list).is_ok(),
        "timeout wrapping terraform must be allowed with multi-word entry"
    );
}
